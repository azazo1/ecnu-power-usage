use anyhow::Context;
use ecnu_power_usage::Records;

mod config;
mod error;
mod log;

use error::Result;
use tauri::State;

use crate::config::{AppState, GuiConfig};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
async fn get_records() -> Records {
    todo!()
}

#[tauri::command]
async fn configure(
    app_state: State<'_, AppState>,
    new_config: GuiConfig,
) -> std::result::Result<(), String> {
    app_state
        .set_config(new_config)
        .await
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> anyhow::Result<()> {
    let _guard = log::init()
        .await
        .with_context(|| "failed to initalize log")?;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::load().await?)
        .invoke_handler(tauri::generate_handler![
            crate_version,
            get_records,
            configure
        ])
        .run(tauri::generate_context!())
        .with_context(|| "error launch tauri app")?;
    Ok(())
}
