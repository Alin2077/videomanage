use crate::db;
use crate::metadata;
use crate::models::{ScanProgress, ScanResult};
use chrono::{DateTime, Local};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub const VIDEO_EXTS: &[&str] = &["mp4", "mkv", "avi", "mov", "webm", "flv", "wmv", "m4v", "3gp"];

const DEFAULT_IGNORED: &[&str] = &[".git", "node_modules", "$RECYCLE.BIN", "System Volume Information", ".Trash"];

pub struct ScanContext {
    pub db: Mutex<Connection>,
    pub is_scanning: AtomicBool,
    pub cancelled: AtomicBool,
    pub progress: Mutex<ScanProgress>,
}

impl ScanContext {
    pub fn new(db: Connection) -> Self {
        Self {
            db: Mutex::new(db),
            is_scanning: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            progress: Mutex::new(ScanProgress {
                is_scanning: false,
                progress: 0.0,
                current_path: String::new(),
                scanned_files: 0,
                total_files: 0,
                added: 0,
                updated: 0,
                unchanged: 0,
                errors: Vec::new(),
            }),
        }
    }

    pub fn snapshot(&self) -> ScanProgress {
        let p = self.progress.lock().unwrap();
        let mut s = p.clone();
        s.is_scanning = self.is_scanning.load(Ordering::SeqCst);
        s
    }

    fn set_progress<F: FnOnce(&mut ScanProgress)>(&self, f: F) {
        let mut p = self.progress.lock().unwrap();
        f(&mut p);
    }

    fn log_error(&self, file: &str, msg: &str) {
        self.set_progress(|p| {
            if p.errors.len() < 200 {
                p.errors.push(format!("{}: {}", file, msg));
            }
        });
        if let Ok(conn) = self.db.lock() {
            let _ = conn.execute(
                "INSERT INTO scan_errors (file_path, message) VALUES (?1, ?2)",
                params![file, msg],
            );
        }
    }
}

fn is_video_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => VIDEO_EXTS.contains(&ext.to_lowercase().as_str()),
        None => false,
    }
}

fn mtime_str(path: &Path) -> String {
    match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(t) => {
            let dt: DateTime<Local> = t.into();
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        }
        Err(_) => String::new(),
    }
}

fn is_ignored(name: &str, ignored: &[String]) -> bool {
    ignored.iter().any(|i| i.eq_ignore_ascii_case(name))
}

/// 确保文件夹层级存在于 DB，返回叶子文件夹 id
fn ensure_folder(conn: &Connection, path: &Path, workspace_id: i64) -> Result<i64, String> {
    let norm = normalize_path(path);
    // 已存在则直接返回
    if let Ok(Some(id)) = conn.query_row(
        "SELECT id FROM folders WHERE path = ?1 AND workspace_id = ?2",
        params![norm, workspace_id],
        |r| r.get(0),
    ) {
        return Ok(id);
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let parent_id: Option<i64> = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() && p != path => {
            Some(ensure_folder(conn, p, workspace_id)?)
        }
        _ => None,
    };

    conn.execute(
        "INSERT INTO folders (parent_id, workspace_id, name, path) VALUES (?1, ?2, ?3, ?4)",
        params![parent_id, workspace_id, name, norm],
    )
    .map_err(|e| format!("插入文件夹失败: {e}"))?;
    Ok(conn.last_insert_rowid())
}

/// 将路径统一为小写反斜杠形式（Windows 兼容比较）
fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

/// 读取忽略文件夹设置
fn ignored_dirs(conn: &Connection) -> Vec<String> {
    let mut out = DEFAULT_IGNORED.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    if let Ok(v) = conn.query_row(
        "SELECT value FROM settings WHERE key = 'ignore_folders'",
        [],
        |r| r.get::<_, String>(0),
    ) {
        for part in v.split(',') {
            let p = part.trim();
            if !p.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(p)) {
                out.push(p.to_string());
            }
        }
    }
    out
}

fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| r.get(0)).ok()
}

/// 开始全量扫描（阻塞调用，运行于后台线程）
pub fn run_scan(ctx: &ScanContext, root: &str, workspace_id: i64) -> Result<ScanResult, String> {
    if ctx.is_scanning.swap(true, Ordering::SeqCst) {
        return Err("扫描正在进行中".to_string());
    }
    ctx.cancelled.store(false, Ordering::SeqCst);

    let root_path = PathBuf::from(root);
    if !root_path.is_dir() {
        ctx.is_scanning.store(false, Ordering::SeqCst);
        return Err(format!("路径不存在或不是文件夹: {root}"));
    }

    ctx.set_progress(|p| {
        p.errors.clear();
        p.added = 0;
        p.updated = 0;
        p.unchanged = 0;
        p.current_path = root.to_string();
    });

    let result = (|| -> Result<ScanResult, String> {
        let conn = ctx.db.lock().unwrap();

        let ignored = ignored_dirs(&conn);
        let ffprobe_path = get_setting(&conn, "ffprobe_path");
        let ffmpeg_path = get_setting(&conn, "ffmpeg_path");
        let compute_hash = get_setting(&conn, "compute_hash").as_deref() == Some("1");

        let ffprobe = metadata::find_ffprobe(ffprobe_path.as_deref());
        let ffmpeg = metadata::find_ffmpeg(ffmpeg_path.as_deref());
        let cover_dir = db::cover_dir().ok();

        // 第一遍：统计视频文件总数
        let mut total: u64 = 0;
        for entry in walkdir::WalkDir::new(&root_path).follow_links(false).into_iter().flatten() {
            if !entry.file_type().is_file() || !is_video_file(entry.path()) {
                continue;
            }
            total += 1;
            if ctx.cancelled.load(Ordering::SeqCst) {
                break;
            }
        }
        ctx.set_progress(|p| p.total_files = total);

        let mut added: u64 = 0;
        let mut updated: u64 = 0;
        let mut unchanged: u64 = 0;
        let mut scanned: u64 = 0;
        let mut errors: Vec<String> = Vec::new();

        'outer: for entry in walkdir::WalkDir::new(&root_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if !e.file_type().is_dir() {
                    return true;
                }
                let name = e.file_name().to_string_lossy().to_string();
                !is_ignored(&name, &ignored)
            })
        {
            if ctx.cancelled.load(Ordering::SeqCst) {
                break 'outer;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() || !is_video_file(entry.path()) {
                continue;
            }
            let file_path = entry.path().to_string_lossy().to_string();
            ctx.set_progress(|p| {
                p.current_path = file_path.clone();
                p.scanned_files = scanned;
                let ratio = if total > 0 { scanned as f64 / total as f64 } else { 0.0 };
                p.progress = (ratio * 100.0).min(99.0);
            });

            let result = process_one_file(&conn, entry.path(), workspace_id, &ffprobe, &ffmpeg, cover_dir.as_deref(), compute_hash);
            scanned += 1;
            match result {
                Ok(Outcome::Added) => added += 1,
                Ok(Outcome::Updated) => updated += 1,
                Ok(Outcome::Unchanged) => unchanged += 1,
                Err(e) => {
                    let file = entry.path().to_string_lossy().to_string();
                    ctx.log_error(&file, &e);
                    errors.push(format!("{}: {e}", entry.path().display()));
                }
            }

            // 每 50 个文件同步一次进度
            if scanned % 50 == 0 {
                ctx.set_progress(|p| {
                    p.scanned_files = scanned;
                    p.added = added;
                    p.updated = updated;
                    p.unchanged = unchanged;
                });
            }
        }

        // 清理：删除已不存在的文件记录
        cleanup_missing(&conn, &root_path, workspace_id);

        ctx.set_progress(|p| {
            p.scanned_files = scanned;
            p.added = added;
            p.updated = updated;
            p.unchanged = unchanged;
            p.progress = 100.0;
            p.current_path = String::new();
            p.errors = errors.clone();
        });

        Ok(ScanResult { workspace_id, added, updated, unchanged, errors })
    })();

    ctx.is_scanning.store(false, Ordering::SeqCst);
    ctx.cancelled.store(false, Ordering::SeqCst);
    result
}

enum Outcome {
    Added,
    Updated,
    Unchanged,
}

fn process_one_file(
    conn: &Connection,
    path: &Path,
    workspace_id: i64,
    ffprobe: &Option<PathBuf>,
    ffmpeg: &Option<PathBuf>,
    cover_dir: Option<&Path>,
    compute_hash: bool,
) -> Result<Outcome, String> {
    let file_path_str = path.to_string_lossy().to_string();
    let mtime = mtime_str(path);
    let file_size = std::fs::metadata(path)
        .map_err(|e| format!("读取文件信息失败: {e}"))?
        .len() as i64;

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path_str.clone());

    // 判断是否需要处理
    let existing: Option<(i64, String, i64)> = conn
        .query_row(
            "SELECT id, modified_at, file_size FROM videos WHERE file_path = ?1",
            params![file_path_str],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();

    if let Some((video_id, db_mtime, db_size)) = existing {
        if db_mtime == mtime && db_size == file_size {
            return Ok(Outcome::Unchanged);
        }
        // 文件有变化 → 重新提取元数据更新
        let (meta, cover_path, hash) = extract(path, video_id, ffprobe, ffmpeg, cover_dir, compute_hash);
        conn.execute(
            "UPDATE videos SET file_size = ?1, duration = ?2, width = ?3, height = ?4,
             codec = ?5, fps = ?6, sample_rate = ?7, cover_path = ?8, file_hash = ?9,
             modified_at = ?10, scanned_at = CURRENT_TIMESTAMP WHERE id = ?11",
            params![
                file_size,
                meta.duration,
                meta.width,
                meta.height,
                meta.codec,
                meta.fps,
                meta.sample_rate,
                cover_path,
                hash,
                mtime,
                video_id
            ],
        )
        .map_err(|e| format!("更新视频失败: {e}"))?;
        return Ok(Outcome::Updated);
    }

    // 新增
    let folder_id = ensure_folder(conn, path.parent().unwrap_or(Path::new("")), workspace_id)?;
    let (meta, cover_path, hash) = extract(path, -1, ffprobe, ffmpeg, cover_dir, compute_hash);
    conn.execute(
        "INSERT INTO videos (folder_id, workspace_id, file_name, file_path, file_size, duration, width, height,
         codec, fps, sample_rate, cover_path, file_hash, modified_at, scanned_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, CURRENT_TIMESTAMP)",
        params![
            folder_id,
            workspace_id,
            file_name,
            file_path_str,
            file_size,
            meta.duration,
            meta.width,
            meta.height,
            meta.codec,
            meta.fps,
            meta.sample_rate,
            cover_path,
            hash,
            mtime
        ],
    )
    .map_err(|e| format!("插入视频失败: {e}"))?;
    Ok(Outcome::Added)
}

/// 提取元数据 + 封面 + 哈希。任何一步失败都降级为 None/Err 记录，不中断主流程。
fn extract(
    path: &Path,
    video_id: i64,
    ffprobe: &Option<PathBuf>,
    ffmpeg: &Option<PathBuf>,
    cover_dir: Option<&Path>,
    compute_hash: bool,
) -> (metadata::VideoMeta, Option<String>, Option<String>) {
    let file_str = path.to_string_lossy().to_string();

    let meta = match ffprobe {
        Some(fp) => metadata::probe_video(fp, &file_str).unwrap_or_default(),
        None => metadata::VideoMeta::default(),
    };

    let cover_path: Option<String> = match (ffmpeg, cover_dir) {
        (Some(fm), Some(dir)) => metadata::generate_cover(fm, &file_str, meta.duration, dir, video_id)
            .ok()
            .map(|p| p.to_string_lossy().to_string()),
        _ => None,
    };

    let hash = if compute_hash {
        metadata::file_sha256(&file_str).ok()
    } else {
        None
    };

    (meta, cover_path, hash)
}

/// 清理数据库中已不存在的文件记录
fn cleanup_missing(conn: &Connection, root: &Path, workspace_id: i64) {
    let root_str = normalize_path(root);
    let videos: Vec<(i64, String)> = conn
        .prepare("SELECT id, file_path FROM videos WHERE file_path LIKE ?1 AND workspace_id = ?2")
        .and_then(|mut stmt| {
            stmt.query_map(params![format!("{}%", root_str), workspace_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map(|rows| rows.flatten().collect())
        })
        .unwrap_or_default();

    for (id, path) in videos {
        let p = PathBuf::from(path.replace('\\', "\\"));
        if !p.exists() {
            let _ = conn.execute("DELETE FROM videos WHERE id = ?1", params![id]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_env(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("vfm_{name}_{pid}"));
        let db = std::env::temp_dir().join(format!("vfm_{name}_{pid}.db"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&db);
        std::fs::create_dir_all(&dir).unwrap();
        (dir, db)
    }

    fn make_workspace(conn: &Connection, path: &str) -> i64 {
        conn.execute(
            "INSERT INTO workspaces (name, path) VALUES (?1, ?2)",
            params![path, path],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn scan_adds_videos_recursively_and_skips_non_video() {
        let (dir, db_path) = temp_env("scan_add");
        std::fs::write(dir.join("test.mp4"), b"fake mp4 content").unwrap();
        std::fs::write(dir.join("cover.jpg"), b"image, not video").unwrap();
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("a.mkv"), b"fake mkv").unwrap();
        std::fs::write(sub.join("notes.txt"), b"ignore me").unwrap();

        let conn = crate::db::init_db_at(&db_path).unwrap();
        let ws = make_workspace(&conn, dir.to_str().unwrap());
        let ctx = ScanContext::new(conn);
        let result = run_scan(&ctx, dir.to_str().unwrap(), ws).unwrap();

        assert_eq!(result.added, 2, "应只入库 2 个视频文件");
        assert_eq!(result.unchanged, 0);
        assert_eq!(result.errors.len(), 0);

        let conn = ctx.db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM videos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        // 文件夹层级应包含根目录与子目录
        let sub_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM folders WHERE path = ?1",
                params![normalize_path(&sub)],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sub_count, 1);

        // 视频应有正确的 folder_id 与大小
        let (size, folder_ok): (i64, i64) = conn
            .query_row(
                "SELECT v.file_size, (SELECT COUNT(*) FROM folders f WHERE f.id = v.folder_id) FROM videos v WHERE v.file_name = 'a.mkv'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(size, "fake mkv".len() as i64);
        assert_eq!(folder_ok, 1);

        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn scan_is_incremental_by_mtime() {
        let (dir, db_path) = temp_env("scan_inc");
        let video = dir.join("clip.webm");
        std::fs::write(&video, b"v1").unwrap();

        let conn = crate::db::init_db_at(&db_path).unwrap();
        let ws = make_workspace(&conn, dir.to_str().unwrap());
        let ctx = ScanContext::new(conn);

        let r1 = run_scan(&ctx, dir.to_str().unwrap(), ws).unwrap();
        assert_eq!(r1.added, 1);

        // 未变化的文件应跳过
        let r2 = run_scan(&ctx, dir.to_str().unwrap(), ws).unwrap();
        assert_eq!(r2.added, 0);
        assert_eq!(r2.updated, 0);
        assert_eq!(r2.unchanged, 1);

        // 修改文件后应更新
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(&video, b"v2 longer content").unwrap();
        let r3 = run_scan(&ctx, dir.to_str().unwrap(), ws).unwrap();
        assert_eq!(r3.updated, 1);

        let conn = ctx.db.lock().unwrap();
        let size: i64 = conn
            .query_row("SELECT file_size FROM videos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(size, "v2 longer content".len() as i64);
        drop(conn);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn scan_removes_records_of_deleted_files() {
        let (dir, db_path) = temp_env("scan_del");
        let video = dir.join("gone.mp4");
        std::fs::write(&video, b"data").unwrap();

        let conn = crate::db::init_db_at(&db_path).unwrap();
        let ws = make_workspace(&conn, dir.to_str().unwrap());
        let ctx = ScanContext::new(conn);
        run_scan(&ctx, dir.to_str().unwrap(), ws).unwrap();

        std::fs::remove_file(&video).unwrap();
        run_scan(&ctx, dir.to_str().unwrap(), ws).unwrap();

        let conn = ctx.db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM videos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "文件删除后记录应被清理");
        drop(conn);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&db_path);
    }
}
