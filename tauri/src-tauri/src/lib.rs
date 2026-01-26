use anyhow::Context;

mod config;
mod error;
mod log;
mod online;

use chromiumoxide::BrowserConfig;
use config::AppState;
use error::Result;

use commands::*;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod commands {
    use std::path::PathBuf;

    use chromiumoxide::BrowserConfig;
    use chrono::{DateTime, FixedOffset};
    use ecnu_power_usage::{ArchiveMeta, CSError, Records, TimeSpan, client::BrowserExecutor};
    use serde::Serialize;
    use tauri::State;
    use tauri_plugin_dialog::DialogExt;
    use tokio::{fs, sync::oneshot};
    use tracing::{error, info};

    use crate::{
        config::{self, ARCHIVE_CACHE_DIRNAME, AppState, GuiConfig},
        online,
    };

    #[tauri::command]
    pub(crate) fn crate_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    #[tauri::command]
    pub(crate) async fn update_config(
        app_state: State<'_, AppState>,
        config: GuiConfig,
    ) -> Result<(), String> {
        app_state
            .set_config(config)
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub(crate) async fn get_config(app_state: State<'_, AppState>) -> Result<GuiConfig, String> {
        Ok(app_state.config.read().await.clone())
    }

    #[tauri::command]
    pub(crate) async fn get_records(app_state: State<'_, AppState>) -> Result<Records, String> {
        let client = app_state.client.read().await;
        client.get_records().await.map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub(crate) async fn pick_room(
        browser_config: State<'_, BrowserConfig>,
        app_state: State<'_, AppState>,
    ) -> Result<(), String> {
        let be = BrowserExecutor::launch(BrowserConfig::clone(&browser_config))
            .await
            .map_err(|e| format!("failed to launch browser: {e:?}"))?;
        let room = be
            .with(async |be| be.pick_room().await)
            .await
            .map_err(|e| format!("failed to pick room: {e:?}"))?;
        app_state
            .client
            .read()
            .await
            .post_room(&room)
            .await
            .map_err(|e| format!("failed to post room: {e:?}"))
    }

    #[tauri::command]
    pub(crate) async fn login(
        browser_config: State<'_, BrowserConfig>,
        app_state: State<'_, AppState>,
    ) -> Result<(), String> {
        let be = BrowserExecutor::launch(BrowserConfig::clone(&browser_config))
            .await
            .map_err(|e| format!("failed to launch browser: {e:?}"))?;
        let cookies = be
            .with(async |be| be.login_cookies().await)
            .await
            .map_err(|e| format!("failed to pick room: {e:?}"))?;
        app_state
            .client
            .read()
            .await
            .post_cookies(&cookies)
            .await
            .map_err(|e| format!("failed to post cookies: {e:?}"))
    }

    /// 下载 archive, 返回保存的路径和 csv 内容.
    #[tauri::command]
    pub(crate) async fn download_archive(
        app_state: State<'_, AppState>,
        archive_name: String,
    ) -> Result<(PathBuf, Records), String> {
        let archive = app_state
            .client
            .read()
            .await
            .download_archive(&archive_name)
            .await
            .map_err(|e| format!("error downloading archive: {e:?}"))?;
        let cache_dir = config::data_dir()
            .await
            .map_err(|_| "failed to create data directory".to_string())?
            .join(ARCHIVE_CACHE_DIRNAME);
        fs::create_dir_all(&cache_dir).await.ok();
        let archive_file = cache_dir.join(format!("{archive_name}.csv"));
        let csv_content = archive
            .to_csv()
            .await
            .map_err(|e| format!("failed to serialize archive: {e:?}"))?;
        fs::write(&archive_file, &csv_content)
            .await
            .map_err(|e| e.to_string())?;
        Ok((archive_file, archive))
    }

    #[tauri::command]
    pub(crate) async fn list_archives(
        app_state: State<'_, AppState>,
    ) -> Result<Vec<ArchiveMeta>, String> {
        app_state
            .client
            .read()
            .await
            .list_archives()
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub(crate) async fn get_degree(app_state: State<'_, AppState>) -> Result<f32, String> {
        app_state
            .client
            .read()
            .await
            .get_degree()
            .await
            .map_err(|e| e.to_string())
    }

    #[derive(Serialize)]
    #[serde(rename_all = "PascalCase")]
    pub enum HealthStatus {
        Ok,
        NoRoom,
        NotLogin,
        ServerDown,
        NoNet,
    }

    #[tauri::command]
    pub(crate) async fn health_check(
        app_state: State<'_, AppState>,
    ) -> Result<HealthStatus, String> {
        match app_state.client.read().await.get_degree().await {
            Ok(_) => Ok(HealthStatus::Ok),
            Err(ecnu_power_usage::Error::CS(CSError::EcnuNotLogin)) => Ok(HealthStatus::NotLogin),
            Err(ecnu_power_usage::Error::CS(CSError::RoomConfigMissing)) => {
                Ok(HealthStatus::NoRoom)
            }
            Err(ecnu_power_usage::Error::Reqwest(e)) => {
                error!("health check reqwest: {e:?}");
                if online::check(None).await {
                    Ok(HealthStatus::ServerDown)
                } else {
                    Ok(HealthStatus::NoNet)
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[tauri::command]
    pub(crate) async fn create_archive(
        app_state: State<'_, AppState>,
        start_time: Option<DateTime<FixedOffset>>,
        end_time: Option<DateTime<FixedOffset>>,
        name: Option<String>,
    ) -> Result<ArchiveMeta, String> {
        app_state
            .client
            .read()
            .await
            .create_archive(name, TimeSpan::new(start_time, end_time))
            .await
            .map_err(|e| format!("failed to create archive: {e:?}"))
    }

    /// 选择一个文本证书/密钥文件并读取其内容, 仅支持小文件.
    #[tauri::command]
    pub(crate) async fn pick_and_read_cert(app: tauri::AppHandle) -> Result<String, String> {
        let (tx, rx) = oneshot::channel();
        app.dialog()
            .file()
            .add_filter("Certificate/Key", &["pem", "crt", "key", "txt"])
            .pick_file(move |fp| {
                info!("file picked: {fp:?}");
                if let Err(e) = tx.send(fp) {
                    error!("picking file: {e:?}");
                }
            });

        if let Ok(Some(path)) = rx.await {
            let path = path
                .into_path()
                .map_err(|e| format!("path convert failed: {e:?}"))?;
            let meta = fs::metadata(&path)
                .await
                .map_err(|e| format!("get file meta failed: {e:?}"))?;
            if meta.len() > 1024 * 50 {
                return Err("file is too large".to_string());
            }
            fs::read_to_string(&path)
                .await
                .map_err(|e| format!("reading failed: {e:?}"))
        } else {
            Err("cancelled".into())
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> anyhow::Result<()> {
    let _guard = log::init()
        .await
        .with_context(|| "failed to initalize log")?;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            pick_and_read_cert
        ])
        .run(tauri::generate_context!())
        .with_context(|| "error launch tauri app")?;
    Ok(())
}
