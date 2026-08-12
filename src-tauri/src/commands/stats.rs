use super::with_db;
use super::TauriState;
use crate::models::{DashboardStats, HourCell, LeaderboardItem, TagStat, TrendPoint};

/// 仪表盘总览统计
#[tauri::command]
pub fn get_dashboard_stats(state: TauriState<'_>) -> Result<DashboardStats, String> {
    with_db(&state, |conn| {
        let total_videos: i64 = conn
            .query_row("SELECT COUNT(*) FROM videos", [], |r| r.get(0))
            .unwrap_or(0);
        let total_folders: i64 = conn
            .query_row("SELECT COUNT(*) FROM folders", [], |r| r.get(0))
            .unwrap_or(0);
        let total_open_count: i64 = conn
            .query_row("SELECT COALESCE(SUM(open_count), 0) FROM videos", [], |r| r.get(0))
            .unwrap_or(0);
        let total_watch_seconds: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(duration), 0) FROM open_logs WHERE status != 'active'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let total_file_size: i64 = conn
            .query_row("SELECT COALESCE(SUM(file_size), 0) FROM videos", [], |r| r.get(0))
            .unwrap_or(0);
        let today_watch_seconds: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(duration), 0) FROM open_logs WHERE status != 'active' AND date(open_time) = date('now','localtime')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let today_open_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM open_logs WHERE date(open_time) = date('now','localtime')",
                [],
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

/// 观看趋势（day: 最近30天 / week: 最近12周 / month: 最近12月）
#[tauri::command]
pub fn get_view_trend(state: TauriState<'_>, range: String) -> Result<Vec<TrendPoint>, String> {
    with_db(&state, |conn| {
        let (group_expr, label_expr, start_expr): (&str, &str, &str) = match range.as_str() {
            "week" => (
                "strftime('%Y-W%W', open_time)",
                "strftime('%Y-W%W', open_time)",
                "date('now','localtime','-77 days')",
            ),
            "month" => (
                "strftime('%Y-%m', open_time)",
                "strftime('%Y-%m', open_time)",
                "date('now','localtime','-11 months','start of month')",
            ),
            _ => (
                "date(open_time)",
                "date(open_time)",
                "date('now','localtime','-29 days')",
            ),
        };
        let sql = format!(
            "SELECT {label_expr} AS label,
                    COALESCE(SUM(CASE WHEN status != 'active' THEN duration ELSE 0 END), 0) AS secs,
                    COUNT(*) AS cnt
             FROM open_logs
             WHERE open_time >= {start_expr}
             GROUP BY {group_expr}
             ORDER BY label ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
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

/// 排行榜
/// category: "open"（最多打开）| "duration"（最长观看）| "recent"（最近活跃）
#[tauri::command]
pub fn get_leaderboard(
    state: TauriState<'_>,
    category: String,
    limit: u32,
) -> Result<Vec<LeaderboardItem>, String> {
    with_db(&state, |conn| {
        let lim = limit.clamp(1, 100) as i64;
        let sql = match category.as_str() {
            "duration" => format!(
                "SELECT v.id, v.file_name, v.file_path, COALESCE(SUM(l.duration),0), v.open_count, v.duration
                 FROM open_logs l JOIN videos v ON v.id = l.video_id
                 WHERE l.status != 'active'
                 GROUP BY v.id ORDER BY 4 DESC LIMIT {lim}"
            ),
            "recent" => format!(
                "SELECT v.id, v.file_name, v.file_path, 0, v.open_count, v.duration
                 FROM open_logs l JOIN videos v ON v.id = l.video_id
                 GROUP BY v.id ORDER BY MAX(l.open_time) DESC LIMIT {lim}"
            ),
            _ => format!(
                "SELECT v.id, v.file_name, v.file_path, v.open_count, v.open_count, v.duration
                 FROM videos v ORDER BY v.open_count DESC LIMIT {lim}"
            ),
        };
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
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

/// 标签分布统计（饼图）
#[tauri::command]
pub fn get_tag_stats(state: TauriState<'_>) -> Result<Vec<TagStat>, String> {
    with_db(&state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.name, t.color, COUNT(vt.video_id) AS cnt
                 FROM tags t LEFT JOIN video_tags vt ON vt.tag_id = t.id
                 GROUP BY t.id ORDER BY cnt DESC, t.name",
            )
            .map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
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

/// 7x24 观看热力图（weekday 0=周一）
#[tauri::command]
pub fn get_hourly_heatmap(state: TauriState<'_>) -> Result<Vec<HourCell>, String> {
    with_db(&state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT (CAST(strftime('%w', open_time) AS INTEGER) + 6) % 7 AS weekday,
                        CAST(strftime('%H', open_time) AS INTEGER) AS hour,
                        COUNT(*) AS cnt,
                        COALESCE(SUM(CASE WHEN status != 'active' THEN duration ELSE 0 END), 0) AS secs
                 FROM open_logs
                 GROUP BY weekday, hour",
            )
            .map_err(|e| format!("查询失败: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
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
