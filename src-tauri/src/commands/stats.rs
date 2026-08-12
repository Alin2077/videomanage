use super::with_db;
use super::TauriState;
use crate::models::{DashboardStats, HourCell, LeaderboardItem, TagStat, TrendPoint};

/// 组装工作区过滤条件（Some(id) 过滤，None 表示全局）
fn ws_where(workspace_id: Option<i64>, table_alias: &str) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    match workspace_id {
        Some(id) => (
            format!("{table_alias}.workspace_id = ?"),
            vec![Box::new(id)],
        ),
        None => (String::new(), Vec::new()),
    }
}

/// 仪表盘总览统计（按工作区过滤）
#[tauri::command]
pub fn get_dashboard_stats(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
) -> Result<DashboardStats, String> {
    with_db(&state, |conn| {
        let (ws_where_v, ws_params_v) = ws_where(workspace_id, "v");
        let (ws_where_l, ws_params_l) = ws_where(workspace_id, "v");

        let video_where = if ws_where_v.is_empty() { String::new() } else { format!("WHERE {ws_where_v}") };
        let log_join_where = if ws_where_l.is_empty() {
            String::new()
        } else {
            format!("AND {ws_where_l}")
        };

        let total_videos: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM videos v {video_where}"),
                rusqlite::params_from_iter(ws_params_v.iter().map(|b| b.as_ref())),
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_folders: i64 = conn
            .query_row("SELECT COUNT(*) FROM folders", [], |r| r.get(0))
            .unwrap_or(0);
        let total_open_count: i64 = conn
            .query_row(
                &format!("SELECT COALESCE(SUM(v.open_count), 0) FROM videos v {video_where}"),
                rusqlite::params_from_iter(ws_params_v.iter().map(|b| b.as_ref())),
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_watch_seconds: f64 = conn
            .query_row(
                &format!(
                    "SELECT COALESCE(SUM(l.duration), 0) FROM open_logs l JOIN videos v ON v.id = l.video_id {log_join_where} WHERE l.status != 'active'"
                ),
                rusqlite::params_from_iter(ws_params_l.iter().map(|b| b.as_ref())),
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let total_file_size: i64 = conn
            .query_row(
                &format!("SELECT COALESCE(SUM(v.file_size), 0) FROM videos v {video_where}"),
                rusqlite::params_from_iter(ws_params_v.iter().map(|b| b.as_ref())),
                |r| r.get(0),
            )
            .unwrap_or(0);
        let today_watch_seconds: f64 = conn
            .query_row(
                &format!(
                    "SELECT COALESCE(SUM(l.duration), 0) FROM open_logs l JOIN videos v ON v.id = l.video_id {log_join_where} WHERE l.status != 'active' AND date(l.open_time) = date('now','localtime')"
                ),
                rusqlite::params_from_iter(ws_params_l.iter().map(|b| b.as_ref())),
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let today_open_count: i64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM open_logs l JOIN videos v ON v.id = l.video_id {log_join_where} WHERE date(l.open_time) = date('now','localtime')"
                ),
                rusqlite::params_from_iter(ws_params_l.iter().map(|b| b.as_ref())),
                |r| r.get(0),
            )
            .unwrap_or(0);

        Ok(DashboardStats {
            total_videos,
            total_folders,
            total_open_count,
            total_watch_seconds,
            total_file_size,
            today_watch_seconds,
            today_open_count,
        })
    })
}

/// 观看趋势（day: 最近30天 / week: 最近12周 / month: 最近12月），按工作区过滤
#[tauri::command]
pub fn get_view_trend(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
    range: String,
) -> Result<Vec<TrendPoint>, String> {
    with_db(&state, |conn| {
        let (group_expr, label_expr, start_expr): (&str, &str, &str) = match range.as_str() {
            "week" => (
                "strftime('%Y-W%W', l.open_time)",
                "strftime('%Y-W%W', l.open_time)",
                "date('now','localtime','-77 days')",
            ),
            "month" => (
                "strftime('%Y-%m', l.open_time)",
                "strftime('%Y-%m', l.open_time)",
                "date('now','localtime','-11 months','start of month')",
            ),
            _ => (
                "date(l.open_time)",
                "date(l.open_time)",
                "date('now','localtime','-29 days')",
            ),
        };
        let (ws_where, ws_params) = ws_where(workspace_id, "v");
        let ws_sql = if ws_where.is_empty() {
            String::new()
        } else {
            format!("AND {ws_where}")
        };
        let sql = format!(
            "SELECT {label_expr} AS label,
                    COALESCE(SUM(CASE WHEN l.status != 'active' THEN l.duration ELSE 0 END), 0) AS secs,
                    COUNT(*) AS cnt
             FROM open_logs l JOIN videos v ON v.id = l.video_id
             WHERE l.open_time >= {start_expr} {ws_sql}
             GROUP BY {group_expr}
             ORDER BY label ASC"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(ws_params.iter().map(|b| b.as_ref())), |r| {
                Ok(TrendPoint {
                    label: r.get(0)?,
                    watch_seconds: r.get(1)?,
                    open_count: r.get(2)?,
                })
            })
            .map_err(|e| format!("查询失败: {e}"))?;
        let points = rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("读取失败: {e}"))?;
        Ok(points)
    })
}

/// 排行榜（category: open | duration | recent），按工作区过滤
#[tauri::command]
pub fn get_leaderboard(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
    category: String,
    limit: u32,
) -> Result<Vec<LeaderboardItem>, String> {
    with_db(&state, |conn| {
        let lim = limit.clamp(1, 100) as i64;
        let (ws_where, ws_params) = ws_where(workspace_id, "v");
        let ws_sql = if ws_where.is_empty() {
            String::new()
        } else {
            format!("AND {ws_where}")
        };
        let sql = match category.as_str() {
            "duration" => format!(
                "SELECT v.id, v.file_name, v.file_path, COALESCE(SUM(l.duration),0), v.open_count, v.duration
                 FROM open_logs l JOIN videos v ON v.id = l.video_id
                 WHERE l.status != 'active' {ws_sql}
                 GROUP BY v.id ORDER BY 4 DESC LIMIT {lim}"
            ),
            "recent" => format!(
                "SELECT v.id, v.file_name, v.file_path, 0, v.open_count, v.duration
                 FROM open_logs l JOIN videos v ON v.id = l.video_id
                 WHERE 1=1 {ws_sql}
                 GROUP BY v.id ORDER BY MAX(l.open_time) DESC LIMIT {lim}"
            ),
            _ => format!(
                "SELECT v.id, v.file_name, v.file_path, v.open_count, v.open_count, v.duration
                 FROM videos v WHERE 1=1 {ws_sql}
                 ORDER BY v.open_count DESC LIMIT {lim}"
            ),
        };
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(ws_params.iter().map(|b| b.as_ref())), |r| {
                Ok(LeaderboardItem {
                    video_id: r.get(0)?,
                    file_name: r.get(1)?,
                    file_path: r.get(2)?,
                    value: r.get(3)?,
                    open_count: r.get(4)?,
                    duration: r.get(5)?,
                })
            })
            .map_err(|e| format!("查询失败: {e}"))?;
        let items = rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("读取失败: {e}"))?;
        Ok(items)
    })
}

/// 标签分布统计（饼图），按工作区过滤
#[tauri::command]
pub fn get_tag_stats(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
) -> Result<Vec<TagStat>, String> {
    with_db(&state, |conn| {
        let (ws_where, ws_params) = ws_where(workspace_id, "v");
        let ws_sql = if ws_where.is_empty() {
            String::new()
        } else {
            format!("AND {ws_where}")
        };
        let sql = format!(
            "SELECT t.id, t.name, t.color, COUNT(vt.video_id) AS cnt
             FROM tags t
             LEFT JOIN video_tags vt ON vt.tag_id = t.id
             LEFT JOIN videos v ON v.id = vt.video_id
             WHERE 1=1 {ws_sql}
             GROUP BY t.id ORDER BY cnt DESC, t.name"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(ws_params.iter().map(|b| b.as_ref())), |r| {
                Ok(TagStat {
                    tag_id: r.get(0)?,
                    tag_name: r.get(1)?,
                    color: r.get(2)?,
                    video_count: r.get(3)?,
                })
            })
            .map_err(|e| format!("查询失败: {e}"))?;
        let items = rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("读取失败: {e}"))?;
        Ok(items)
    })
}

/// 7x24 观看热力图（weekday 0=周一），按工作区过滤
#[tauri::command]
pub fn get_hourly_heatmap(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
) -> Result<Vec<HourCell>, String> {
    with_db(&state, |conn| {
        let (ws_where, ws_params) = ws_where(workspace_id, "v");
        let ws_sql = if ws_where.is_empty() {
            String::new()
        } else {
            format!("AND {ws_where}")
        };
        let sql = format!(
            "SELECT (CAST(strftime('%w', l.open_time) AS INTEGER) + 6) % 7 AS weekday,
                    CAST(strftime('%H', l.open_time) AS INTEGER) AS hour,
                    COUNT(*) AS cnt,
                    COALESCE(SUM(CASE WHEN l.status != 'active' THEN l.duration ELSE 0 END), 0) AS secs
             FROM open_logs l JOIN videos v ON v.id = l.video_id
             WHERE 1=1 {ws_sql}
             GROUP BY weekday, hour"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(ws_params.iter().map(|b| b.as_ref())), |r| {
                Ok(HourCell {
                    weekday: r.get(0)?,
                    hour: r.get(1)?,
                    count: r.get(2)?,
                    seconds: r.get(3)?,
                })
            })
            .map_err(|e| format!("查询失败: {e}"))?;
        let items = rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("读取失败: {e}"))?;
        Ok(items)
    })
}
