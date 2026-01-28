use anyhow::Context;

mod commands;
mod config;
mod error;
mod log;
mod online;

use chromiumoxide::BrowserConfig;
use commands::*;
use config::AppState;
use error::{Error, Result};

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
            sys_notify
        ])
        .run(tauri::generate_context!())
        .with_context(|| "error launch tauri app")?;
    Ok(())
}
