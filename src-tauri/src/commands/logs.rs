use super::with_db;
use super::TauriState;
use crate::models::{LogFilter, OpenLog};
use rusqlite::params;

/// 记录视频打开事件（返回 log_id），并递增 open_count
#[tauri::command]
pub fn log_video_open(state: TauriState<'_>, video_id: i64) -> Result<i64, String> {
    with_db(&state, |conn| {
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
        Ok(log_id)
    })
}

/// 记录视频关闭事件（计算观看时长）
#[tauri::command]
pub fn log_video_close(state: TauriState<'_>, log_id: i64) -> Result<(), String> {
    with_db(&state, |conn| {
        let open_time: String = conn
            .query_row(
                "SELECT open_time FROM open_logs WHERE id = ?1",
                params![log_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("查询日志失败: {e}"))?;
        conn.execute(
            "UPDATE open_logs SET close_time = datetime('now','localtime'),
             duration = CAST((julianday('now','localtime') - julianday(?1)) * 86400 AS REAL),
             status = 'closed' WHERE id = ?2 AND status = 'active'",
            params![open_time, log_id],
        )
        .map_err(|e| format!("更新日志失败: {e}"))?;
        Ok(())
    })
}

/// 修复异常关闭的日志（程序启动时调用）
#[tauri::command]
pub fn repair_crashed_logs(state: TauriState<'_>) -> Result<u64, String> {
    with_db(&state, |conn| {
        let n = conn
            .execute(
                "UPDATE open_logs SET close_time = datetime('now','localtime'),
                 duration = CAST((julianday('now','localtime') - julianday(open_time)) * 86400 AS REAL),
                 status = 'crashed' WHERE status = 'active' AND close_time IS NULL",
                [],
            )
            .map_err(|e| format!("修复日志失败: {e}"))?;
        Ok(n as u64)
    })
}

/// 查询打开日志（按日期范围 / 视频 / 工作区筛选）
#[tauri::command]
pub fn list_logs(
    state: TauriState<'_>,
    filter: LogFilter,
    workspace_id: Option<i64>,
    page: u32,
    page_size: u32,
) -> Result<crate::models::PageResult<crate::models::OpenLogWithVideo>, String> {
    with_db(&state, |conn| {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ws) = workspace_id {
            where_clauses.push("v.workspace_id = ?".to_string());
            params_vec.push(Box::new(ws));
        }
        if let Some(vid) = filter.video_id {
            where_clauses.push("l.video_id = ?".to_string());
            params_vec.push(Box::new(vid));
        }
        if let Some(sd) = filter.start_date.as_deref().filter(|s| !s.is_empty()) {
            where_clauses.push("date(l.open_time) >= date(?)".to_string());
            params_vec.push(Box::new(sd.to_string()));
        }
        if let Some(ed) = filter.end_date.as_deref().filter(|s| !s.is_empty()) {
            where_clauses.push("date(l.open_time) <= date(?)".to_string());
            params_vec.push(Box::new(ed.to_string()));
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM open_logs l {where_sql}");
        let total: i64 = conn
            .prepare(&count_sql)
            .and_then(|mut stmt| {
                stmt.query_row(
                    rusqlite::params_from_iter(params_vec.iter().map(|b| b.as_ref())),
                    |r| r.get(0),
                )
            })
            .map_err(|e| format!("查询失败: {e}"))?;

        let page = page.max(1);
        let size = page_size.clamp(1, 500);
        let offset = (page - 1) * size;

        let sql = format!(
            "SELECT l.id, l.video_id, l.open_time, l.close_time, l.duration, l.status,
                    v.file_name, v.file_path
             FROM open_logs l JOIN videos v ON v.id = l.video_id
             {where_sql} ORDER BY l.open_time DESC LIMIT ? OFFSET ?"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
        let refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(refs.into_iter().chain([
                    &size as &dyn rusqlite::ToSql,
                    &offset as &dyn rusqlite::ToSql,
                ])),
                |r| {
                    Ok(crate::models::OpenLogWithVideo {
                        log: OpenLog {
                            id: r.get(0)?,
                            video_id: r.get(1)?,
                            open_time: r.get(2)?,
                            close_time: r.get(3)?,
                            duration: r.get(4)?,
                            status: r.get(5)?,
                        },
                        file_name: r.get(6)?,
                        file_path: r.get(7)?,
                    })
                },
            )
            .map_err(|e| format!("查询失败: {e}"))?;
        let items = rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("读取失败: {e}"))?;
        Ok(crate::models::PageResult { total, items })
    })
}

/// 导出日志为 CSV
#[tauri::command]
pub fn export_logs(
    state: TauriState<'_>,
    filter: LogFilter,
    workspace_id: Option<i64>,
    output_path: String,
) -> Result<(), String> {
    with_db(&state, |conn| {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(ws) = workspace_id {
            where_clauses.push("v.workspace_id = ?".to_string());
            params_vec.push(Box::new(ws));
        }
        if let Some(vid) = filter.video_id {
            where_clauses.push("l.video_id = ?".to_string());
            params_vec.push(Box::new(vid));
        }
        if let Some(sd) = filter.start_date.as_deref().filter(|s| !s.is_empty()) {
            where_clauses.push("date(l.open_time) >= date(?)".to_string());
            params_vec.push(Box::new(sd.to_string()));
        }
        if let Some(ed) = filter.end_date.as_deref().filter(|s| !s.is_empty()) {
            where_clauses.push("date(l.open_time) <= date(?)".to_string());
            params_vec.push(Box::new(ed.to_string()));
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let sql = format!(
            "SELECT v.file_name, v.file_path, l.open_time, l.close_time, l.duration, l.status
             FROM open_logs l JOIN videos v ON v.id = l.video_id
             {where_sql} ORDER BY l.open_time"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
        let mut rows = stmt
            .query(rusqlite::params_from_iter(params_vec.iter().map(|b| b.as_ref())))
            .map_err(|e| format!("查询失败: {e}"))?;

        let mut csv = String::from("视频名,路径,打开时间,关闭时间,观看时长(秒),状态\n");
        while let Some(row) = rows.next().map_err(|e| format!("读取失败: {e}"))? {
            let name: String = row.get(0).unwrap_or_default();
            let path: String = row.get(1).unwrap_or_default();
            let open: String = row.get(2).unwrap_or_default();
            let close: Option<String> = row.get(3).unwrap_or_default();
            let duration: Option<f64> = row.get(4).unwrap_or_default();
            let status: String = row.get(5).unwrap_or_default();
            csv.push_str(&format!(
                "\"{}\",\"{}\",{},{},{},{}\n",
                name.replace('"', "\"\""),
                path.replace('"', "\"\""),
                open,
                close.unwrap_or_default(),
                duration.map(|d| format!("{d:.1}")).unwrap_or_default(),
                status
            ));
        }
        std::fs::write(&output_path, csv).map_err(|e| format!("写入 CSV 失败: {e}"))?;
        Ok(())
    })
}
