use super::with_db;
use super::TauriState;
use crate::models::{DashboardStats, HourCell, LeaderboardItem, TagStat, TrendPoint};
use rusqlite::Connection;

/// 组装工作区过滤条件（Some(id) 过滤，None 表示全局）
fn ws_where(
    workspace_id: Option<i64>,
    table_alias: &str,
) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    match workspace_id {
        Some(id) => (
            format!("{table_alias}.workspace_id = ?"),
            vec![Box::new(id)],
        ),
        None => (String::new(), Vec::new()),
    }
}

fn params_from(
    vec: &[Box<dyn rusqlite::ToSql>],
) -> rusqlite::ParamsFromIter<std::vec::IntoIter<&dyn rusqlite::ToSql>> {
    let refs: Vec<&dyn rusqlite::ToSql> = vec.iter().map(|b| b.as_ref()).collect();
    rusqlite::params_from_iter(refs.into_iter())
}

/// 仪表盘总览统计（按工作区过滤）
pub fn dashboard_stats(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<DashboardStats, String> {
    let (ws_where_v, ws_params_v) = ws_where(workspace_id, "v");
    let video_where = if ws_where_v.is_empty() {
        String::new()
    } else {
        format!("WHERE {ws_where_v}")
    };
    // 日志统计通过 JOIN videos 过滤工作区（ON 子句附带条件）
    let log_join_where = if ws_where_v.is_empty() {
        String::new()
    } else {
        format!("AND {ws_where_v}")
    };

    let total_videos: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM videos v {video_where}"),
            params_from(&ws_params_v),
            |r| r.get(0),
        )
        .unwrap_or(0);

    // 文件夹按工作区过滤
    let total_folders: i64 = match workspace_id {
        Some(id) => conn
            .query_row(
                "SELECT COUNT(*) FROM folders WHERE workspace_id = ?1",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .unwrap_or(0),
        None => conn
            .query_row("SELECT COUNT(*) FROM folders", [], |r| r.get(0))
            .unwrap_or(0),
    };

    let total_open_count: i64 = conn
        .query_row(
            &format!("SELECT COALESCE(SUM(v.open_count), 0) FROM videos v {video_where}"),
            params_from(&ws_params_v),
            |r| r.get(0),
        )
        .unwrap_or(0);

    let total_watch_seconds: f64 = conn
        .query_row(
            &format!(
                "SELECT COALESCE(SUM(l.duration), 0) FROM open_logs l JOIN videos v ON v.id = l.video_id {log_join_where} WHERE l.status != 'active'"
            ),
            params_from(&ws_params_v),
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    let total_file_size: i64 = conn
        .query_row(
            &format!("SELECT COALESCE(SUM(v.file_size), 0) FROM videos v {video_where}"),
            params_from(&ws_params_v),
            |r| r.get(0),
        )
        .unwrap_or(0);

    let today_watch_seconds: f64 = conn
        .query_row(
            &format!(
                "SELECT COALESCE(SUM(l.duration), 0) FROM open_logs l JOIN videos v ON v.id = l.video_id {log_join_where} WHERE l.status != 'active' AND date(l.open_time) = date('now','localtime')"
            ),
            params_from(&ws_params_v),
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    let today_open_count: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM open_logs l JOIN videos v ON v.id = l.video_id {log_join_where} WHERE date(l.open_time) = date('now','localtime')"
            ),
            params_from(&ws_params_v),
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
}

/// 观看趋势（day: 最近30天 / week: 最近12周 / month: 最近12月），按工作区过滤
pub fn view_trend(
    conn: &Connection,
    workspace_id: Option<i64>,
    range: String,
) -> Result<Vec<TrendPoint>, String> {
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
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
    let rows = stmt
        .query_map(params_from(&ws_params), |r| {
            Ok(TrendPoint {
                label: r.get(0)?,
                watch_seconds: r.get(1)?,
                open_count: r.get(2)?,
            })
        })
        .map_err(|e| format!("查询失败: {e}"))?;
    let points = rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("读取失败: {e}"))?;
    Ok(points)
}

/// 排行榜（category: open | duration | recent），按工作区过滤
pub fn leaderboard(
    conn: &Connection,
    workspace_id: Option<i64>,
    category: String,
    limit: u32,
) -> Result<Vec<LeaderboardItem>, String> {
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
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
    let rows = stmt
        .query_map(params_from(&ws_params), |r| {
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
}

/// 标签分布统计（饼图），按工作区过滤；所有标签均返回（计数可能为 0）
pub fn tag_stats(conn: &Connection, workspace_id: Option<i64>) -> Result<Vec<TagStat>, String> {
    let (ws_where, ws_params) = ws_where(workspace_id, "v");
    let ws_on = if ws_where.is_empty() {
        String::new()
    } else {
        format!("AND {ws_where}")
    };
    let sql = format!(
        "SELECT t.id, t.name, t.color, COUNT(v.id) AS cnt
         FROM tags t
         LEFT JOIN video_tags vt ON vt.tag_id = t.id
         LEFT JOIN videos v ON v.id = vt.video_id {ws_on}
         GROUP BY t.id ORDER BY cnt DESC, t.name"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
    let rows = stmt
        .query_map(params_from(&ws_params), |r| {
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
}

/// 7x24 观看热力图（weekday 0=周一），按工作区过滤
pub fn hourly_heatmap(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Vec<HourCell>, String> {
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
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {e}"))?;
    let rows = stmt
        .query_map(params_from(&ws_params), |r| {
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
}

// ---------- Tauri 命令包装 ----------

#[tauri::command]
pub fn get_dashboard_stats(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
) -> Result<DashboardStats, String> {
    with_db(&state, |conn| dashboard_stats(conn, workspace_id))
}

#[tauri::command]
pub fn get_view_trend(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
    range: String,
) -> Result<Vec<TrendPoint>, String> {
    with_db(&state, |conn| view_trend(conn, workspace_id, range))
}

#[tauri::command]
pub fn get_leaderboard(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
    category: String,
    limit: u32,
) -> Result<Vec<LeaderboardItem>, String> {
    with_db(&state, |conn| leaderboard(conn, workspace_id, category, limit))
}

#[tauri::command]
pub fn get_tag_stats(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
) -> Result<Vec<TagStat>, String> {
    with_db(&state, |conn| tag_stats(conn, workspace_id))
}

#[tauri::command]
pub fn get_hourly_heatmap(
    state: TauriState<'_>,
    workspace_id: Option<i64>,
) -> Result<Vec<HourCell>, String> {
    with_db(&state, |conn| hourly_heatmap(conn, workspace_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::sync::atomic::{AtomicU64, Ordering};
    static DB_SEQ: AtomicU64 = AtomicU64::new(0);

    fn setup_db() -> (Connection, i64, i64) {
        // 每个测试使用独立数据库文件（cargo test 并行执行）
        let seq = DB_SEQ.fetch_add(1, Ordering::SeqCst);
        let db = std::env::temp_dir().join(format!("vfm_stats_{}_{}.db", std::process::id(), seq));
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_file(format!("{}-wal", db.display()));
        let _ = std::fs::remove_file(format!("{}-shm", db.display()));
        let conn = crate::db::init_db_at(&db).unwrap();

        // 工作区 A
        conn.execute("INSERT INTO workspaces (name, path) VALUES ('A', 'C:\\A')", []).unwrap();
        let ws_a = conn.last_insert_rowid();
        // 工作区 B
        conn.execute("INSERT INTO workspaces (name, path) VALUES ('B', 'C:\\B')", []).unwrap();
        let ws_b = conn.last_insert_rowid();

        let insert_ws = |conn: &Connection, ws: i64, name: &str, folder: &str| -> i64 {
            conn.execute(
                "INSERT INTO folders (parent_id, workspace_id, name, path) VALUES (NULL, ?1, ?2, ?3)",
                params![ws, name, folder],
            )
            .unwrap();
            conn.last_insert_rowid()
        };

        // A: 文件夹 fa1，2 个视频（一个打标签、一条 closed 日志）
        let fa1 = insert_ws(&conn, ws_a, "fa1", "C:\\A\\fa1");
        conn.execute(
            "INSERT INTO videos (folder_id, workspace_id, file_name, file_path, file_size, duration, open_count)
             VALUES (?1, ?2, 'a1.mp4', 'C:\\A\\fa1\\a1.mp4', 100, 60, 3)",
            params![fa1, ws_a],
        )
        .unwrap();
        let v_a1 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO videos (folder_id, workspace_id, file_name, file_path, file_size, duration, open_count)
             VALUES (?1, ?2, 'a2.mp4', 'C:\\A\\fa1\\a2.mp4', 200, 120, 0)",
            params![fa1, ws_a],
        )
        .unwrap();

        // B: 文件夹 fb1，1 个视频
        let fb1 = insert_ws(&conn, ws_b, "fb1", "C:\\B\\fb1");
        conn.execute(
            "INSERT INTO videos (folder_id, workspace_id, file_name, file_path, file_size, duration, open_count)
             VALUES (?1, ?2, 'b1.mp4', 'C:\\B\\fb1\\b1.mp4', 300, 30, 5)",
            params![fb1, ws_b],
        )
        .unwrap();

        // 标签
        conn.execute("INSERT INTO tag_groups (name) VALUES ('类型')", []).unwrap();
        let g = conn.last_insert_rowid();
        conn.execute("INSERT INTO tags (group_id, name) VALUES (?1, '电影')", params![g]).unwrap();
        let tag = conn.last_insert_rowid();
        conn.execute("INSERT INTO video_tags (video_id, tag_id) VALUES (?1, ?2)", params![v_a1, tag])
            .unwrap();

        // 日志：A 的一条 closed 日志（60 秒）
        conn.execute(
            "INSERT INTO open_logs (video_id, open_time, close_time, duration, status)
             VALUES (?1, datetime('now','localtime','-2 hours'), datetime('now','localtime','-1 hours'), 3600, 'closed')",
            params![v_a1],
        )
        .unwrap();

        (conn, ws_a, ws_b)
    }

    #[test]
    fn dashboard_filters_by_workspace() {
        let (conn, ws_a, ws_b) = setup_db();

        let a = dashboard_stats(&conn, Some(ws_a)).unwrap();
        assert_eq!(a.total_videos, 2, "A 应有 2 个视频");
        assert_eq!(a.total_open_count, 3, "A 的 open_count 总和为 3");
        assert_eq!(a.total_folders, 1, "A 应有 1 个文件夹");
        assert_eq!(a.total_watch_seconds, 3600.0, "A 的观看时长应为 3600 秒");
        assert_eq!(a.total_file_size, 300, "A 的文件大小总和为 100+200");
        assert_eq!(a.today_open_count, 1, "A 今日播放 1 次");

        let b = dashboard_stats(&conn, Some(ws_b)).unwrap();
        assert_eq!(b.total_videos, 1, "B 应有 1 个视频");
        assert_eq!(b.total_open_count, 5);
        assert_eq!(b.total_watch_seconds, 0.0, "B 无日志");

        let all = dashboard_stats(&conn, None).unwrap();
        assert_eq!(all.total_videos, 3, "全局应有 3 个视频");
        assert_eq!(all.total_open_count, 8);
    }

    #[test]
    fn trend_and_leaderboard_filter_by_workspace() {
        let (conn, ws_a, ws_b) = setup_db();

        let trend_a = view_trend(&conn, Some(ws_a), "day".into()).unwrap();
        assert_eq!(trend_a.len(), 1, "A 有 1 天有日志");
        assert_eq!(trend_a[0].open_count, 1);

        let trend_b = view_trend(&conn, Some(ws_b), "day".into()).unwrap();
        assert_eq!(trend_b.len(), 0, "B 无日志");

        let lb_a = leaderboard(&conn, Some(ws_a), "open".into(), 10).unwrap();
        assert_eq!(lb_a.len(), 2, "A 的打开排行有 2 个视频");
        assert_eq!(lb_a[0].file_name, "a1.mp4");

        let lb_b = leaderboard(&conn, Some(ws_b), "open".into(), 10).unwrap();
        assert_eq!(lb_b.len(), 1);
        assert_eq!(lb_b[0].file_name, "b1.mp4");
        assert_eq!(lb_b[0].open_count, 5);

        // 时长排行只含 closed 日志的视频（A 的 a1）
        let lb_dur = leaderboard(&conn, Some(ws_a), "duration".into(), 10).unwrap();
        assert_eq!(lb_dur.len(), 1);
        assert_eq!(lb_dur[0].value, 3600.0);
    }

    #[test]
    fn tag_stats_and_heatmap_filter_by_workspace() {
        let (conn, ws_a, ws_b) = setup_db();

        let tags_a = tag_stats(&conn, Some(ws_a)).unwrap();
        assert_eq!(tags_a.len(), 1, "A 的标签计数：电影 1 个");
        assert_eq!(tags_a[0].video_count, 1);

        let tags_b = tag_stats(&conn, Some(ws_b)).unwrap();
        assert_eq!(tags_b.len(), 1, "B 无标签视频，计数为 0");
        assert_eq!(tags_b[0].video_count, 0);

        let hm_a = hourly_heatmap(&conn, Some(ws_a)).unwrap();
        assert_eq!(hm_a.len(), 1, "A 有 1 个热力格");
        assert_eq!(hm_a[0].count, 1);

        let hm_b = hourly_heatmap(&conn, Some(ws_b)).unwrap();
        assert_eq!(hm_b.len(), 0, "B 无热力数据");
    }
}
