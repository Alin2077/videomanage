use super::TauriState;
use crate::models::{FolderNode, ScanProgress, ScanResult, Workspace};
use crate::scanner;
use rusqlite::params;

/// 新增（或复用已有路径的）工作区并启动全量扫描（后台执行）
#[tauri::command]
pub async fn scan_root_folder(
    state: TauriState<'_>,
    path: String,
    name: String,
) -> Result<ScanResult, String> {
    if path.trim().is_empty() {
        return Err("扫描路径不能为空".to_string());
    }
    if !std::path::Path::new(&path).is_dir() {
        return Err(format!("路径不存在或不是文件夹: {path}"));
    }

    // 创建或复用工作区（按 path 唯一）
    let norm_path = path.replace('/', "\\");
    let workspace_id = {
        let conn = state
            .scan
            .db
            .lock()
            .map_err(|_| "数据库锁获取失败".to_string())?;
        if let Ok(Some(id)) = conn.query_row(
            "SELECT id FROM workspaces WHERE path = ?1",
            params![norm_path],
            |r| r.get(0),
        ) {
            id
        } else {
            let ws_name = if name.trim().is_empty() {
                std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone())
            } else {
                name.trim().to_string()
            };
            conn.execute(
                "INSERT INTO workspaces (name, path) VALUES (?1, ?2)",
                params![ws_name, norm_path],
            )
            .map_err(|e| format!("创建工作区失败: {e}"))?;
            conn.last_insert_rowid()
        }
    };

    let ctx = state.scan.clone();
    tauri::async_runtime::spawn_blocking(move || scanner::run_scan(&ctx, &path, workspace_id))
        .await
        .map_err(|e| format!("扫描任务异常: {e}"))?
}

/// 列出全部工作区（含视频数与文件夹数）
#[tauri::command]
pub fn list_workspaces(state: TauriState<'_>) -> Result<Vec<Workspace>, String> {
    let conn = state
        .scan
        .db
        .lock()
        .map_err(|_| "数据库锁获取失败".to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT w.id, w.name, w.path, w.created_at,
                    (SELECT COUNT(*) FROM videos v WHERE v.workspace_id = w.id) AS video_count,
                    (SELECT COUNT(*) FROM folders f WHERE f.workspace_id = w.id) AS folder_count
             FROM workspaces w
             ORDER BY w.id",
        )
        .map_err(|e| format!("查询失败: {e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Workspace {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                created_at: r.get(3)?,
                video_count: r.get(4)?,
                folder_count: r.get(5)?,
            })
        })
        .map_err(|e| format!("查询失败: {e}"))?;
    let workspaces = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取失败: {e}"))?;
    Ok(workspaces)
}

/// 删除工作区（级联删除其文件夹与视频记录）
#[tauri::command]
pub fn delete_workspace(state: TauriState<'_>, workspace_id: i64) -> Result<(), String> {
    let conn = state
        .scan
        .db
        .lock()
        .map_err(|_| "数据库锁获取失败".to_string())?;
    conn.execute("DELETE FROM workspaces WHERE id = ?1", params![workspace_id])
        .map_err(|e| format!("删除工作区失败: {e}"))?;
    Ok(())
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

/// 获取指定工作区的根文件夹
#[tauri::command]
pub fn get_root_folders(state: TauriState<'_>, workspace_id: i64) -> Result<Vec<FolderNode>, String> {
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
             WHERE f.parent_id IS NULL AND f.workspace_id = ?1
             ORDER BY f.name",
        )
        .map_err(|e| format!("查询失败: {e}"))?;
    let rows = stmt
        .query_map(params![workspace_id], |r| {
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
