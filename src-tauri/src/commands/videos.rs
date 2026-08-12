use super::attach_tags;
use super::row_to_video;
use super::video_tags;
use super::with_db;
use super::DbRef;
use super::TauriState;
use super::VIDEO_COLUMNS;
use crate::models::{
    PageResult, VideoDetail, VideoInfo, VideoMetaUpdate, VideoQuery,
};
use rusqlite::{params, OptionalExtension};

fn sort_column(sort_by: Option<&str>) -> &'static str {
    match sort_by {
        Some("name") => "v.file_name",
        Some("size") => "v.file_size",
        Some("duration") => "v.duration",
        Some("open_count") => "v.open_count",
        Some("modified_at") => "v.modified_at",
        _ => "v.scanned_at",
    }
}

fn sort_order(order: Option<&str>) -> &'static str {
    match order {
        Some("asc") => "ASC",
        _ => "DESC",
    }
}

/// 获取视频列表（分页 + 筛选 + 排序）
#[tauri::command]
pub fn list_videos(state: TauriState<'_>, query: VideoQuery) -> Result<PageResult<VideoInfo>, String> {
    with_db(&state, |conn| {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // 工作区过滤（视频库页面总是携带当前工作区）
        if let Some(ws) = query.workspace_id {
            where_clauses.push("v.workspace_id = ?".to_string());
            params_vec.push(Box::new(ws));
        }
        if let Some(fid) = query.folder_id {
            where_clauses.push("v.folder_id = ?".to_string());
            params_vec.push(Box::new(fid));
        }
        if let Some(kw) = query.keyword.as_deref().filter(|k| !k.trim().is_empty()) {
            where_clauses.push("(v.file_name LIKE ? OR v.file_path LIKE ? OR v.custom_title LIKE ? OR v.notes LIKE ?)".to_string());
            let like = format!("%{}%", kw.trim());
            params_vec.push(Box::new(like.clone()));
            params_vec.push(Box::new(like.clone()));
            params_vec.push(Box::new(like.clone()));
            params_vec.push(Box::new(like));
        }
        if let Some(tag_ids) = query.tag_ids.as_deref().filter(|t| !t.is_empty()) {
            let marks = vec!["?"; tag_ids.len()].join(",");
            where_clauses.push(format!(
                "v.id IN (SELECT video_id FROM video_tags WHERE tag_id IN ({marks}))"
            ));
            for t in tag_ids {
                params_vec.push(Box::new(*t));
            }
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        // 总数
        let count_sql = format!("SELECT COUNT(*) FROM videos v {where_sql}");
        let total: i64 = {
            let mut stmt = conn
                .prepare(&count_sql)
                .map_err(|e| format!("查询失败: {e}"))?;
            let count = stmt
                .query_row(params_from(&params_vec), |r| r.get(0))
                .map_err(|e| format!("查询失败: {e}"))?;
            count
        };

        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 200);
        let offset = (page - 1) * page_size;

        let sql = format!(
            "SELECT {VIDEO_COLUMNS} FROM videos v {where_sql}
             ORDER BY {} {} LIMIT ? OFFSET ?",
            sort_column(query.sort_by.as_deref()),
            sort_order(query.sort_order.as_deref())
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("查询失败: {e}"))?;

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();
        let mut videos: Vec<VideoInfo> = stmt
            .query_map(
                rusqlite::params_from_iter(param_refs.iter().copied().chain([
                    &page_size as &dyn rusqlite::ToSql,
                    &offset as &dyn rusqlite::ToSql,
                ])),
                |r| row_to_video(r),
            )
            .map_err(|e| format!("查询失败: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取失败: {e}"))?;

        attach_tags(conn, &mut videos)?;
        Ok(PageResult { total, items: videos })
    })
}

fn params_from(
    vec: &[Box<dyn rusqlite::ToSql>],
) -> rusqlite::ParamsFromIter<std::vec::IntoIter<&dyn rusqlite::ToSql>> {
    let refs: Vec<&dyn rusqlite::ToSql> = vec.iter().map(|b| b.as_ref()).collect();
    rusqlite::params_from_iter(refs.into_iter())
}

/// 获取视频详情（含标签 + 播放历史）
#[tauri::command]
pub fn get_video_detail(state: TauriState<'_>, video_id: i64) -> Result<VideoDetail, String> {
    with_db(&state, |conn| {
        let info: VideoInfo = conn
            .query_row(
                &format!("SELECT {VIDEO_COLUMNS} FROM videos v WHERE v.id = ?1"),
                params![video_id],
                |r| row_to_video(r),
            )
            .optional()
            .map_err(|e| format!("查询失败: {e}"))?
            .ok_or_else(|| format!("视频不存在: {video_id}"))?;

        let mut info = info;
        info.tags = video_tags(conn, video_id)?;

        let folder_path: String = conn
            .query_row(
                "SELECT path FROM folders WHERE id = ?1",
                params![info.folder_id],
                |r| r.get(0),
            )
            .unwrap_or_default();

        let logs: Vec<crate::models::OpenLog> = {
            let mut stmt = conn
                .prepare(
                    "SELECT id, video_id, open_time, close_time, duration, status
                     FROM open_logs WHERE video_id = ?1 ORDER BY open_time DESC LIMIT 200",
                )
                .map_err(|e| format!("查询日志失败: {e}"))?;
            let rows = stmt
                .query_map(params![video_id], |r| {
                    Ok(crate::models::OpenLog {
                        id: r.get(0)?,
                        video_id: r.get(1)?,
                        open_time: r.get(2)?,
                        close_time: r.get(3)?,
                        duration: r.get(4)?,
                        status: r.get(5)?,
                    })
                })
                .map_err(|e| format!("查询日志失败: {e}"))?;
            let result: Vec<crate::models::OpenLog> = rows
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("读取日志失败: {e}"))?;
            result
        };

        Ok(VideoDetail { info, folder_path, open_logs: logs })
    })
}

/// 更新视频元信息（备注、自定义标题）
#[tauri::command]
pub fn update_video_meta(
    state: TauriState<'_>,
    video_id: i64,
    meta: VideoMetaUpdate,
) -> Result<(), String> {
    with_db(&state, |conn| {
        conn.execute(
            "UPDATE videos SET custom_title = COALESCE(?1, custom_title),
             notes = COALESCE(?2, notes) WHERE id = ?3",
            params![meta.custom_title, meta.notes, video_id],
        )
        .map_err(|e| format!("更新失败: {e}"))?;
        Ok(())
    })
}

/// 批量删除视频记录（级联删除标签关联与日志）
#[tauri::command]
pub fn batch_delete_videos(state: TauriState<'_>, video_ids: Vec<i64>) -> Result<u64, String> {
    with_db(&state, |conn| {
        let tx = conn.unchecked_transaction().map_err(|e| format!("事务失败: {e}"))?;
        let mut count = 0u64;
        for id in &video_ids {
            // 尝试删除封面文件
            if let Ok(cover) = tx
                .query_row("SELECT cover_path FROM videos WHERE id = ?1", params![id], |r| r.get::<_, String>(0))
            {
                let _ = std::fs::remove_file(&cover);
            }
            let n = tx
                .execute("DELETE FROM videos WHERE id = ?1", params![id])
                .map_err(|e| format!("删除失败: {e}"))?;
            count += n as u64;
        }
        tx.commit().map_err(|e| format!("提交失败: {e}"))?;
        Ok(count)
    })
}

/// 搜索视频（文件名/路径/备注/标签）
#[tauri::command]
pub fn search_videos(state: TauriState<'_>, keyword: String, limit: u32) -> Result<Vec<VideoInfo>, String> {
    let kw = keyword.trim();
    if kw.is_empty() {
        return Ok(Vec::new());
    }
    with_db(&state, |conn| {
        let like = format!("%{kw}%");
        let sql = format!(
            "SELECT DISTINCT {VIDEO_COLUMNS} FROM videos v
             LEFT JOIN video_tags vt ON vt.video_id = v.id
             LEFT JOIN tags t ON t.id = vt.tag_id
             WHERE v.file_name LIKE ?1 OR v.file_path LIKE ?2
                OR v.custom_title LIKE ?3 OR v.notes LIKE ?4 OR t.name LIKE ?5
             ORDER BY v.open_count DESC LIMIT ?6"
        );
        let limit_i = limit.min(500) as i64;
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map(
                params![like, like, like, like, like, limit_i],
                |r| row_to_video(r),
            )
            .map_err(|e| format!("查询失败: {e}"))?;
        let mut videos: Vec<VideoInfo> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for row in rows {
            let v = row.map_err(|e| format!("读取失败: {e}"))?;
            if seen.insert(v.id) {
                videos.push(v);
            }
        }
        attach_tags(conn, &mut videos)?;
        Ok(videos)
    })
}

/// 使用外部播放器打开视频并记录观看日志（播放器退出时自动写关闭日志）
#[tauri::command]
pub fn open_with_player(
    state: TauriState<'_>,
    video_id: i64,
    player_path: Option<String>,
) -> Result<i64, String> {
    // 创建打开日志（status=active），open_count +1
    let file_path = with_db(&state, |conn| {
        let fp: String = conn
            .query_row(
                "SELECT file_path FROM videos WHERE id = ?1",
                params![video_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("查询视频失败: {e}"))?;
        conn.execute(
            "INSERT INTO open_logs (video_id, open_time, status) VALUES (?1, datetime('now','localtime'), 'active')",
            params![video_id],
        )
        .map_err(|e| format!("写入日志失败: {e}"))?;
        let log_id = conn.last_insert_rowid();
        conn.execute(
            "UPDATE videos SET open_count = open_count + 1 WHERE id = ?1",
            params![video_id],
        )
        .map_err(|e| format!("更新计数失败: {e}"))?;
        Ok::<(String, i64), String>((fp, log_id))
    })?;

    // 确定播放器：参数 > 设置 > 系统默认
    let player = if let Some(p) = player_path.filter(|p| !p.trim().is_empty()) {
        Some(p)
    } else {
        with_db(&state, |conn| {
            conn.query_row(
                "SELECT value FROM settings WHERE key = 'player_path'",
                [],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| format!("查询设置失败: {e}"))
        })
        .ok()
    };

    let result = match player {
        Some(p) => {
            let child = std::process::Command::new(&p)
                .arg(&file_path.0)
                .spawn()
                .map_err(|e| format!("启动播放器失败: {e}"))?;
            // 后台等待播放器退出，自动记录关闭日志
            let ctx = state.scan.clone();
            let log_id = file_path.1;
            std::thread::spawn(move || {
                let mut child = child;
                let _ = child.wait();
                close_log_inner(&ctx, log_id);
            });
            Ok(file_path.1)
        }
        None => {
            // 无配置播放器：用系统默认程序打开（无法检测退出，由前端关闭时调用 log_video_close）
            open_with_system(&file_path.0)?;
            Ok(file_path.1)
        }
    };
    result
}

#[cfg(target_os = "windows")]
fn open_with_system(path: &str) -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", path])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("调用系统默认程序失败: {e}"))
}

#[cfg(not(target_os = "windows"))]
fn open_with_system(path: &str) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    std::process::Command::new(opener)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("调用系统默认程序失败: {e}"))
}

fn close_log_inner(state: &DbRef, log_id: i64) {
    let _ = (|| -> Result<(), String> {
        let conn = state.db.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        let open_time: String = conn
            .query_row(
                "SELECT open_time FROM open_logs WHERE id = ?1",
                params![log_id],
                |r| r.get(0),
            )
            .unwrap_or_default();
        conn.execute(
            "UPDATE open_logs SET close_time = datetime('now','localtime'),
             duration = CAST((julianday('now','localtime') - julianday(?1)) * 86400 AS REAL),
             status = 'closed' WHERE id = ?2 AND status = 'active'",
            params![open_time, log_id],
        )
        .map(|_| ())
        .map_err(|e| format!("更新日志失败: {e}"))
    })();
}
