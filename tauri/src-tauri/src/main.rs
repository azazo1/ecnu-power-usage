// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ecnu_power_usage_gui_lib::run().await
}
