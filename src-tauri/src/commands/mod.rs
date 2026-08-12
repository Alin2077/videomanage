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
