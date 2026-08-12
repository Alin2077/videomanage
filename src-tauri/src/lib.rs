mod commands;
mod db;
mod metadata;
mod models;
mod scanner;

use commands::{logs, scan, settings, stats, tags, videos};
use scanner::ScanContext;
use std::sync::Arc;

pub struct AppState {
    pub scan: Arc<ScanContext>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 应用待导入的备份（若存在）
    if let Ok(true) = db::apply_pending_import() {
        println!("已应用导入的备份");
    }

    let conn = match db::init_db() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("数据库初始化失败: {e}");
            std::process::exit(1);
        }
    };

    // 修复上次异常关闭的日志
    {
        let ctx = ScanContext::new(conn);
        let _ = ctx
            .db
            .lock()
            .unwrap()
            .execute(
                "UPDATE open_logs SET close_time = datetime('now','localtime'),
                 duration = CAST((julianday('now','localtime') - julianday(open_time)) * 86400 AS REAL),
                 status = 'crashed' WHERE status = 'active' AND close_time IS NULL",
                [],
            );
        let state = AppState { scan: Arc::new(ctx) };

        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_dialog::init())
            .manage(state)
            .invoke_handler(tauri::generate_handler![
                // 扫描
                scan::scan_root_folder,
                scan::get_scan_progress,
                scan::cancel_scan,
                scan::get_folder_children,
                scan::get_root_folders,
                // 视频
                videos::list_videos,
                videos::get_video_detail,
                videos::update_video_meta,
                videos::batch_delete_videos,
                videos::search_videos,
                videos::open_with_player,
                // 标签
                tags::get_tag_tree,
                tags::upsert_tag,
                tags::delete_tag,
                tags::upsert_tag_group,
                tags::delete_tag_group,
                tags::set_video_tags,
                tags::batch_add_tags,
                tags::batch_remove_tags,
                // 日志
                logs::log_video_open,
                logs::log_video_close,
                logs::repair_crashed_logs,
                logs::list_logs,
                logs::export_logs,
                // 统计
                stats::get_dashboard_stats,
                stats::get_view_trend,
                stats::get_leaderboard,
                stats::get_tag_stats,
                stats::get_hourly_heatmap,
                // 设置
                settings::get_settings,
                settings::set_setting,
                settings::detect_media_tools,
                settings::export_backup,
                settings::import_backup,
            ])
            .run(tauri::generate_context!())
            .expect("运行 Tauri 应用失败");
    }
}
