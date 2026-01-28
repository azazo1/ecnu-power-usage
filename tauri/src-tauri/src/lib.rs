use anyhow::Context;

mod commands;
mod config;
mod error;
mod log;
mod online;
mod tray;

use chromiumoxide::BrowserConfig;
use commands::*;
use config::AppState;
use error::{Error, Result};
use tauri::WindowEvent;
use tracing::warn;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> anyhow::Result<()> {
    let _guard = log::init()
        .await
        .with_context(|| "failed to initalize log")?;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::load().await?)
        .manage(BrowserConfig::builder().with_head().build().unwrap())
        .invoke_handler(tauri::generate_handler![
            crate_version,
            update_config,
            get_config,
            get_records,
            pick_room,
            login,
            download_archive,
            list_archives,
            get_degree,
            health_check,
            create_archive,
            pick_cert,
            sys_notify,
            delete_archive
        ])
        .setup(|app| Ok(tray::init_tray(app)?))
        .on_window_event(|window, evt| {
            if window.label() != "main" {
                return;
            }
            match evt {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    if let Err(e) = window.hide() {
                        warn!("failed to hide window: {e}.");
                        window.minimize().ok();
                    };

                    // 窗口隐藏后, 从 Dock 移除图标
                    #[cfg(target_os = "macos")]
                    {
                        use objc2::MainThreadMarker;
                        use objc2_app_kit::NSApplication;
                        use objc2_app_kit::NSApplicationActivationPolicy;

                        // 获取当前应用实例
                        let app =
                            NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
                        // 设置为 Accessory 模式（即从 Dock 移除，但在托盘可见）
                        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
                    }
                }
                WindowEvent::Focused(true) => {
                    // 窗口获取焦点, 从 Dock 恢复图标
                    #[cfg(target_os = "macos")]
                    {
                        use objc2::MainThreadMarker;
                        use objc2_app_kit::NSApplication;
                        use objc2_app_kit::NSApplicationActivationPolicy;

                        let app =
                            NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
                        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .with_context(|| "error launch tauri app")?;
    Ok(())
}
