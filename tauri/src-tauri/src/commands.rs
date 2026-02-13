// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::path::PathBuf;

use chromiumoxide::BrowserConfig;
use chrono::{DateTime, FixedOffset};
use ecnu_power_usage::{
    ArchiveMeta, CSError, Cookies, Records, TimeSpan, client::BrowserExecutor, config::RoomConfig,
    rooms::RoomInfo,
};
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;
use tokio::{fs, sync::oneshot};
use tracing::{error, info};

use crate::{
    config::{self, ARCHIVE_CACHE_DIRNAME, AppState, GuiConfig},
    routine::health::HealthStatus,
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
    let (room, cookies): (RoomConfig, ecnu_power_usage::Result<Cookies>) = be
        .with(async |be| Ok((be.pick_room().await?, be.login_cookies().await)))
        .await
        .map_err(|e| format!("failed to pick room: {e:?}"))?;

    let client = app_state.client.read().await;
    client
        .post_room(&room)
        .await
        .map_err(|e| format!("failed to post room: {e:?}"))?;
    if let Ok(cookies) = cookies {
        client.post_cookies(&cookies).await.ok();
    }
    Ok(())
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

#[tauri::command]
pub(crate) async fn health_check(app_state: State<'_, AppState>) -> Result<HealthStatus, String> {
    app_state.health.read().await.clone()
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
        .map_err(|e| {
            error!("create archive: {e:?}");
            if let ecnu_power_usage::Error::CS(cse) = e {
                match cse {
                    CSError::EmptyArchive
                    | CSError::ArchiveDir
                    | CSError::ListArchive
                    | CSError::WriteArchive
                    | CSError::DuplicatedArchive
                    | CSError::InvalidArchiveName
                    | CSError::ArchiveNotFound => cse.to_string(),
                    _ => "failed to create archive".to_string(),
                }
            } else {
                "failed to create archive".to_string()
            }
        })
}

/// 选择一个文本证书/密钥文件路径.
#[tauri::command]
pub(crate) async fn pick_cert(app: tauri::AppHandle) -> Result<String, String> {
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
        let path = path.into_path().map_err(|e| {
            error!("picked path convert: {e:?}");
            "path convert failed.".to_string()
        })?;
        path.to_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "invalid utf-8 path".to_string())
    } else {
        Err("cancelled".into())
    }
}

/// 发送系统通知.
#[tauri::command]
pub(crate) fn sys_notify(
    app: tauri::AppHandle,
    title: String,
    message: String,
) -> Result<(), String> {
    #[cfg(feature = "display-detecting")]
    {
        use crate::display::all_displays_asleep;

        if all_displays_asleep().is_ok_and(|x| x) {
            return Ok(());
        }
    }

    app.notification()
        .builder()
        .title(&title)
        .body(&message)
        .show()
        .map_err(|e| {
            error!(target: "notification error", "{e:?}");
            format!("notification error: {e}")
        })
}

#[tauri::command]
pub(crate) async fn delete_archive(
    app_state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    app_state
        .client
        .read()
        .await
        .delete_archive(name)
        .await
        .map_err(|e| {
            error!("delete archive failed: {e:?}");
            format!("delete archive failed: {e}")
        })
}

#[tauri::command]
pub(crate) async fn clear_room(app_state: State<'_, AppState>) -> Result<(), String> {
    app_state
        .client
        .read()
        .await
        .clear_room()
        .await
        .map_err(|e| format!("clearing room failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn clear_cookies(app_state: State<'_, AppState>) -> Result<(), String> {
    app_state
        .client
        .read()
        .await
        .clear_cookies()
        .await
        .map_err(|e| format!("clearing cookeis failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_room_info(app_state: State<'_, AppState>) -> Result<RoomInfo, String> {
    let client = app_state.client.read().await;
    client.get_room_info().await.map_err(|e| {
        error!("get room info failed: {e:?}");
        format!("get room info failed: {e}")
    })
}

#[tauri::command]
pub(crate) async fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
