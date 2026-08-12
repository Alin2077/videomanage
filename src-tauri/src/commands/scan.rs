use super::TauriState;
use crate::models::{FolderNode, ScanProgress, ScanResult};
use crate::scanner;
use rusqlite::params;

/// 选择根文件夹并启动全量扫描（后台执行）
#[tauri::command]
pub async fn scan_root_folder(
    state: TauriState<'_>,
    path: String,
) -> Result<ScanResult, String> {
    if path.trim().is_empty() {
        return Err("扫描路径不能为空".to_string());
    }
    if !std::path::Path::new(&path).is_dir() {
        return Err(format!("路径不存在或不是文件夹: {path}"));
    }
    let ctx = state.scan.clone();
    tauri::async_runtime::spawn_blocking(move || scanner::run_scan(&ctx, &path))
        .await
        .map_err(|e| format!("扫描任务异常: {e}"))?
}

/// 获取扫描进度
#[tauri::command]
pub fn get_scan_progress(state: TauriState<'_>) -> Result<ScanProgress, String> {
    Ok(state.scan.snapshot())
}

/// 取消扫描
#[tauri::command]
pub fn cancel_scan(state: TauriState<'_>) -> Result<(), String> {
    state
        .scan
        .cancelled
        .store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// 获取文件夹树子节点（懒加载）
#[tauri::command]
pub fn get_folder_children(
    state: TauriState<'_>,
    parent_id: Option<i64>,
) -> Result<Vec<FolderNode>, String> {
    let conn = state
        .scan
        .db
        .lock()
        .map_err(|_| "数据库锁获取失败".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.parent_id, f.name, f.path,
                    (SELECT COUNT(*) FROM videos v WHERE v.folder_id = f.id) AS video_count
             FROM folders f
             WHERE f.parent_id IS ?1
             ORDER BY f.name",
        )
        .map_err(|e| format!("查询失败: {e}"))?;

    let rows = stmt
        .query_map(params![parent_id], |r| {
            let id: i64 = r.get(0)?;
            let parent: Option<i64> = r.get(1)?;
            let name: String = r.get(2)?;
            let path: String = r.get(3)?;
            let video_count: i64 = r.get(4)?;
            Ok(FolderNode { id, parent_id: parent, name, path, video_count, has_children: false })
        })
        .map_err(|e| format!("查询失败: {e}"))?;

    let mut nodes: Vec<FolderNode> = Vec::new();
    for row in rows {
        nodes.push(row.map_err(|e| format!("读取失败: {e}"))?);
    }
    // stmt 已随作用域结束释放，可再次借用 conn 判断是否有子文件夹
    for n in nodes.iter_mut() {
        n.has_children = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE parent_id = ?1)",
                params![n.id],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false);
    }
    Ok(nodes)
}

/// 获取根文件夹
#[tauri::command]
pub fn get_root_folders(state: TauriState<'_>) -> Result<Vec<FolderNode>, String> {
    let conn = state
        .scan
        .db
        .lock()
        .map_err(|_| "数据库锁获取失败".to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.parent_id, f.name, f.path,
                    (SELECT COUNT(*) FROM videos v WHERE v.folder_id = f.id) AS video_count,
                    EXISTS(SELECT 1 FROM folders c WHERE c.parent_id = f.id) AS has_children
             FROM folders f
             WHERE f.parent_id IS NULL
             ORDER BY f.name",
        )
        .map_err(|e| format!("查询失败: {e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(FolderNode {
                id: r.get(0)?,
                parent_id: r.get(1)?,
                name: r.get(2)?,
                path: r.get(3)?,
                video_count: r.get(4)?,
                has_children: r.get(5)?,
            })
        })
        .map_err(|e| format!("查询失败: {e}"))?;
    let nodes = rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("读取失败: {e}"))?;
    Ok(nodes)
}
