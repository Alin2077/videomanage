use super::with_db;
use super::TauriState;
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// 获取全部设置
#[tauri::command]
pub fn get_settings(state: TauriState<'_>) -> Result<HashMap<String, String>, String> {
    with_db(&state, |conn| {
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| format!("查询失败: {e}"))?;
        let map: HashMap<String, String> = rows
            .collect::<Result<_, _>>()
            .map_err(|e| format!("读取失败: {e}"))?;
        Ok(map)
    })
}

/// 设置单个配置项
#[tauri::command]
pub fn set_setting(state: TauriState<'_>, key: String, value: String) -> Result<(), String> {
    with_db(&state, |conn| {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            params![key, value],
        )
        .map_err(|e| format!("保存设置失败: {e}"))?;
        Ok(())
    })
}

/// 探测系统中可用的 ffprobe / ffmpeg（优先已配置路径，其次 PATH）
#[tauri::command]
pub fn detect_media_tools(state: TauriState<'_>) -> Result<serde_json::Value, String> {
    let conn = state
        .scan
        .db
        .lock()
        .map_err(|_| "数据库锁获取失败".to_string())?;
    let ffprobe_configured = get_setting(&conn, "ffprobe_path");
    let ffmpeg_configured = get_setting(&conn, "ffmpeg_path");
    Ok(serde_json::json!({
        "ffprobe": crate::metadata::find_ffprobe(ffprobe_configured.as_deref()),
        "ffmpeg": crate::metadata::find_ffmpeg(ffmpeg_configured.as_deref()),
    }))
}

fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .ok()
}

/// 导出备份（数据库 + 封面缓存打包为 .vfm-backup）
#[tauri::command]
pub fn export_backup(state: TauriState<'_>, output_path: String) -> Result<(), String> {
    // 先 checkpoint WAL，确保数据落盘
    with_db(&state, |conn| {
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| format!("checkpoint 失败: {e}"))?;
        Ok(())
    })?;

    let db_file = crate::db::db_path()?;
    let covers = crate::db::cover_dir()?;

    let file = std::fs::File::create(&output_path)
        .map_err(|e| format!("创建备份文件失败: {e}"))?;
    let mut builder = tar::Builder::new(file);
    builder
        .append_path_with_name(&db_file, "videos.db")
        .map_err(|e| format!("写入数据库失败: {e}"))?;

    if covers.is_dir() {
        for entry in std::fs::read_dir(&covers).map_err(|e| format!("读取封面目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取封面目录失败: {e}"))?;
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                builder
                    .append_path_with_name(entry.path(), format!("covers/{name}"))
                    .map_err(|e| format!("写入封面失败: {e}"))?;
            }
        }
    }
    builder
        .finish()
        .map_err(|e| format!("完成打包失败: {e}"))?;
    Ok(())
}

/// 导入备份：解压 .vfm-backup 到暂存目录，应用重启时自动生效
#[tauri::command]
pub fn import_backup(state: TauriState<'_>, input_path: String) -> Result<(), String> {
    let db_file = crate::db::db_path()?;
    let data_dir = db_file.parent().ok_or("无法确定数据目录")?.to_path_buf();
    let pending_dir = data_dir.join(".import-pending");

    // 解压到暂存目录
    if pending_dir.exists() {
        std::fs::remove_dir_all(&pending_dir).map_err(|e| format!("清理暂存目录失败: {e}"))?;
    }
    std::fs::create_dir_all(&pending_dir).map_err(|e| format!("创建暂存目录失败: {e}"))?;

    let file = std::fs::File::open(&input_path)
        .map_err(|e| format!("打开备份文件失败: {e}"))?;
    let mut archive = tar::Archive::new(file);
    for entry in archive.entries().map_err(|e| format!("读取备份失败: {e}"))? {
        let mut entry = entry.map_err(|e| format!("读取备份条目失败: {e}"))?;
        let path = entry.path().map_err(|e| format!("读取备份路径失败: {e}"))?.to_string_lossy().to_string();
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut buf).map_err(|e| format!("读取备份内容失败: {e}"))?;
        let out_path = pending_dir.join(&path);
        if let Some(parent) = out_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&out_path, buf).map_err(|e| format!("写入暂存文件失败: {e}"))?;
    }

    if !pending_dir.join("videos.db").is_file() {
        let _ = std::fs::remove_dir_all(&pending_dir);
        return Err("备份文件中缺少 videos.db".to_string());
    }
    // 解锁 DB（避免影响后续写操作；导入在重启时应用）
    drop(state.scan.db.lock().map_err(|_| "数据库锁获取失败".to_string())?);
    let _ = data_dir.join("videos.db-wal");
    let _ = data_dir.join("videos.db-shm");
    Ok(())
}
