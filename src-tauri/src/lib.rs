mod commands;
mod db;
mod metadata;
mod models;
mod scanner;

use commands::{logs, scan, settings, stats, tags, videos};
use scanner::ScanContext;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

pub struct AppState {
    pub scan: Arc<ScanContext>,
}

/// 更新托盘 tooltip 为简略统计信息（hover 显示）
fn update_tray_tooltip(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let (workspaces, videos, today) = if let Ok(conn) = state.scan.db.lock() {
        let ws: i64 = conn
            .query_row("SELECT COUNT(*) FROM workspaces", [], |r| r.get(0))
            .unwrap_or(0);
        let v: i64 = conn
            .query_row("SELECT COUNT(*) FROM videos", [], |r| r.get(0))
            .unwrap_or(0);
        let t: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM open_logs WHERE date(open_time) = date('now','localtime')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        (ws, v, t)
    } else {
        (0, 0, 0)
    };
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&format!(
            "视频文件管理\n工作区 {workspaces} · 视频 {videos} · 今日播放 {today}"
        )));
    }
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
            .setup(|app| {
                let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &quit])?;
                let _tray = TrayIconBuilder::new()
                    .icon(app.default_window_icon().expect("缺少默认应用图标").clone())
                    .tooltip("视频文件管理")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    })
                    .build(app)?;
                update_tray_tooltip(app.handle());
                Ok(())
            })
            .on_window_event(|window, event| {
                // 关闭行为：设置 close_to_tray=1 时最小化到系统托盘
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let close_to_tray = {
                        let state = window.state::<AppState>();
                        let value: Option<String> = {
                            let conn = state.scan.db.lock().ok();
                            match conn {
                                Some(c) => c
                                    .query_row(
                                        "SELECT value FROM settings WHERE key = 'close_to_tray'",
                                        [],
                                        |r| r.get::<_, String>(0),
                                    )
                                    .ok(),
                                None => None,
                            }
                        };
                        value.unwrap_or_default() == "1"
                    };
                    if close_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                        update_tray_tooltip(window.app_handle());
                    }
                }
            })
            .invoke_handler(tauri::generate_handler![
                // 扫描与工作区
                scan::scan_root_folder,
                scan::list_workspaces,
                scan::delete_workspace,
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
