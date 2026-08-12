pub mod logs;
pub mod scan;
pub mod settings;
pub mod stats;
pub mod tags;
pub mod videos;

use crate::models::VideoInfo;
use crate::scanner::ScanContext;
use rusqlite::{params, Connection};
use std::sync::Arc;

pub type DbRef = Arc<ScanContext>;
pub type TauriState<'a> = tauri::State<'a, crate::AppState>;

pub fn with_db<'a, T>(
    state: &TauriState<'a>,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let conn = state
        .scan
        .db
        .lock()
        .map_err(|_| "数据库锁获取失败".to_string())?;
    f(&conn)
}

/// 为一批视频批量加载标签（避免 N+1）
pub fn attach_tags(conn: &Connection, videos: &mut [VideoInfo]) -> Result<(), String> {
    if videos.is_empty() {
        return Ok(());
    }
    let ids: Vec<String> = videos.iter().map(|v| v.id.to_string()).collect();
    let in_clause = ids.join(",");
    let sql = format!(
        "SELECT vt.video_id, t.id, t.group_id, t.name, t.color
         FROM video_tags vt JOIN tags t ON t.id = vt.tag_id
         WHERE vt.video_id IN ({in_clause}) ORDER BY t.name"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("查询标签失败: {e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, crate::models::Tag {
                id: r.get(1)?,
                group_id: r.get(2)?,
                name: r.get(3)?,
                color: r.get(4)?,
            }))
        })
        .map_err(|e| format!("查询标签失败: {e}"))?;

    use std::collections::HashMap;
    let mut map: HashMap<i64, Vec<crate::models::Tag>> = HashMap::new();
    for row in rows {
        let (vid, tag) = row.map_err(|e| format!("读取标签失败: {e}"))?;
        map.entry(vid).or_default().push(tag);
    }
    for v in videos.iter_mut() {
        v.tags = map.remove(&v.id).unwrap_or_default();
    }
    Ok(())
}

/// 计算每个目标文件夹的子树视频数（含其所有子孙文件夹的视频）。
/// 用于树状结构节点计数，与列表的递归过滤保持一致。
pub fn subtree_video_counts(
    conn: &Connection,
    folder_ids: &[i64],
) -> Result<std::collections::HashMap<i64, i64>, String> {
    use std::collections::HashMap;
    let mut result = HashMap::new();
    if folder_ids.is_empty() {
        return Ok(result);
    }

    // 1. 收集目标文件夹及其所有后代 id
    let mut all: Vec<i64> = folder_ids.to_vec();
    let mut frontier: Vec<i64> = folder_ids.to_vec();
    loop {
        let ph = vec!["?"; frontier.len()].join(",");
        let sql = format!("SELECT id FROM folders WHERE parent_id IN ({ph})");
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
        let kids: Vec<i64> = stmt
            .query_map(rusqlite::params_from_iter(frontier.iter()), |r| r.get(0))
            .map_err(|e| format!("查询失败: {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("读取失败: {e}"))?;
        if kids.is_empty() {
            break;
        }
        frontier = kids.clone();
        all.extend(kids);
    }

    // 2. 直接视频数（folder_id 精确匹配）
    let ph = vec!["?"; all.len()].join(",");
    let sql = format!("SELECT folder_id, COUNT(*) FROM videos WHERE folder_id IN ({ph}) GROUP BY folder_id");
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
    let mut direct: HashMap<i64, i64> = HashMap::new();
    let rows = stmt
        .query_map(rusqlite::params_from_iter(all.iter()), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("查询失败: {e}"))?;
    for row in rows {
        let (fid, cnt) = row.map_err(|e| format!("读取失败: {e}"))?;
        direct.insert(fid, cnt);
    }

    // 3. 父子关系（仅关注 all 集合内）
    let ph = vec!["?"; all.len()].join(",");
    let sql = format!("SELECT id, parent_id FROM folders WHERE id IN ({ph})");
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
    let mut children: HashMap<i64, Vec<i64>> = HashMap::new();
    let rows = stmt
        .query_map(rusqlite::params_from_iter(all.iter()), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?))
        })
        .map_err(|e| format!("查询失败: {e}"))?;
    for row in rows {
        let (id, parent) = row.map_err(|e| format!("读取失败: {e}"))?;
        if let Some(p) = parent {
            children.entry(p).or_default().push(id);
        }
    }

    // 4. 对每个目标文件夹 DFS 求和（含自身与所有后代）
    for &root in folder_ids {
        let mut total: i64 = 0;
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            total += direct.get(&id).copied().unwrap_or(0);
            if let Some(kids) = children.get(&id) {
                stack.extend(kids.iter().copied());
            }
        }
        result.insert(root, total);
    }
    Ok(result)
}

/// 查询单个视频的标签
pub fn video_tags(conn: &Connection, video_id: i64) -> Result<Vec<crate::models::Tag>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.group_id, t.name, t.color FROM tags t
             JOIN video_tags vt ON vt.tag_id = t.id
             WHERE vt.video_id = ?1 ORDER BY t.name",
        )
        .map_err(|e| format!("查询标签失败: {e}"))?;
    let rows = stmt
        .query_map(params![video_id], |r| {
            Ok(crate::models::Tag {
                id: r.get(0)?,
                group_id: r.get(1)?,
                name: r.get(2)?,
                color: r.get(3)?,
            })
        })
        .map_err(|e| format!("查询标签失败: {e}"))?;
    let mut tags = Vec::new();
    for row in rows {
        tags.push(row.map_err(|e| format!("读取标签失败: {e}"))?);
    }
    Ok(tags)
}

/// 从查询行构造 VideoInfo（列顺序固定）
pub fn row_to_video(r: &rusqlite::Row) -> rusqlite::Result<VideoInfo> {
    Ok(VideoInfo {
        id: r.get(0)?,
        folder_id: r.get(1)?,
        file_name: r.get(2)?,
        file_path: r.get(3)?,
        file_size: r.get(4)?,
        duration: r.get(5)?,
        width: r.get(6)?,
        height: r.get(7)?,
        codec: r.get(8)?,
        fps: r.get(9)?,
        sample_rate: r.get(10)?,
        cover_path: r.get(11)?,
        custom_title: r.get(12)?,
        notes: r.get(13)?,
        open_count: r.get(14)?,
        file_hash: r.get(15)?,
        created_at: r.get(16)?,
        modified_at: r.get(17)?,
        scanned_at: r.get(18)?,
        tags: Vec::new(),
    })
}

pub const VIDEO_COLUMNS: &str = "v.id, v.folder_id, v.file_name, v.file_path, v.file_size,
    v.duration, v.width, v.height, v.codec, v.fps, v.sample_rate, v.cover_path,
    v.custom_title, v.notes, v.open_count, v.file_hash, v.created_at, v.modified_at, v.scanned_at";

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn setup() -> (Connection, i64) {
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let db = std::env::temp_dir().join(format!("vfm_mod_{}_{}.db", std::process::id(), seq));
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_file(format!("{}-wal", db.display()));
        let _ = std::fs::remove_file(format!("{}-shm", db.display()));
        let conn = crate::db::init_db_at(&db).unwrap();
        conn.execute("INSERT INTO workspaces (name, path) VALUES ('w', 'C:\\w')", [])
            .unwrap();
        let ws = conn.last_insert_rowid();
        (conn, ws)
    }

    fn insert_video(conn: &Connection, ws: i64, folder_id: i64, name: &str) {
        conn.execute(
            "INSERT INTO videos (folder_id, workspace_id, file_name, file_path, file_size)
             VALUES (?1, ?2, ?3, ?4, 10)",
            params![folder_id, ws, name, format!("C:\\x\\{name}")],
        )
        .unwrap();
    }

    /// 子树视频计数：含所有子孙文件夹的视频
    #[test]
    fn subtree_video_counts_includes_descendants() {
        let (conn, ws) = setup();
        // 根 → 子 → 孙
        conn.execute(
            "INSERT INTO folders (parent_id, workspace_id, name, path) VALUES (NULL, ?1, 'root', 'C:\\root')",
            params![ws],
        )
        .unwrap();
        let root = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO folders (parent_id, workspace_id, name, path) VALUES (?1, ?2, 'sub', 'C:\\root\\sub')",
            params![root, ws],
        )
        .unwrap();
        let sub = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO folders (parent_id, workspace_id, name, path) VALUES (?1, ?2, 'deep', 'C:\\root\\sub\\deep')",
            params![sub, ws],
        )
        .unwrap();
        let deep = conn.last_insert_rowid();

        // 视频分布：root 直属 2、sub 直属 1、deep 直属 3
        insert_video(&conn, ws, root, "r1.mp4");
        insert_video(&conn, ws, root, "r2.mp4");
        insert_video(&conn, ws, sub, "s1.mp4");
        insert_video(&conn, ws, deep, "d1.mp4");
        insert_video(&conn, ws, deep, "d2.mp4");
        insert_video(&conn, ws, deep, "d3.mp4");

        let counts = subtree_video_counts(&conn, &[root, sub, deep]).unwrap();
        assert_eq!(counts[&root], 6, "根应含全部 6 个视频");
        assert_eq!(counts[&sub], 4, "子应含自身+孙 4 个视频");
        assert_eq!(counts[&deep], 3, "孙应含自身 3 个视频");

        // 递归过滤 SQL（list_videos 所用）：选中 sub 应得到 4 条
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM videos v WHERE v.folder_id IN (
                    WITH RECURSIVE subq(id) AS (
                        SELECT ?1 UNION ALL
                        SELECT c.id FROM folders c JOIN subq ON c.parent_id = subq.id
                    )
                    SELECT id FROM subq
                )",
                params![sub],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 4, "递归过滤应返回子目录树全部视频");
    }
}
