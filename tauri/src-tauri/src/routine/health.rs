use std::{error::Error, fmt::Display, time::Duration};

use ecnu_power_usage::CSError;
use serde::Serialize;
use tauri::{Manager, State};
use tracing::{error, info};

use crate::{commands::sys_notify, config::AppState, online};

const NETWORK_TOLERANCE_TIMES: usize = 3;

#[derive(Serialize, Debug, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub(crate) enum HealthStatus {
    Ok,
    NoRoom,
    NotLogin,
    ServerDown,
    NoNet,
    TlsError,
    /// Unknown 仅在这里构建.
    #[serde(skip)]
    Unknown,
}

fn get_notify_content<T: Display>(health: Result<HealthStatus, T>) -> (String, String) {
    match health {
        Ok(HealthStatus::NoNet) => ("网络已断开".into(), "请检查网络设置。".into()),
        Ok(HealthStatus::ServerDown) => ("服务器连接失败".into(), "后端服务暂时无法访问。".into()),
        Ok(HealthStatus::NotLogin) => ("登录已过期".into(), "请重新登录以继续。".into()),
        Ok(HealthStatus::NoRoom) => ("房间未绑定".into(), "请先配置您的宿舍房间号。".into()),
        Ok(HealthStatus::TlsError) => ("安全连接失败".into(), "证书校验无效，连接已终止。".into()),
        Ok(HealthStatus::Ok) => ("系统正常".into(), "系统正常运行".into()),
        Ok(HealthStatus::Unknown) => ("系统错误".into(), "发生未知异常".into()),
        Err(e) => ("系统错误".into(), format!("发生未知异常: {e}")),
    }
}

trait IsTlsError {
    fn is_tls_error(&self) -> bool;
}

impl IsTlsError for reqwest::Error {
    fn is_tls_error(&self) -> bool {
        let mut source = self.source();

        while let Some(err) = source {
            if let Some(hyper_err) = err.downcast_ref::<hyper::Error>()
                && hyper_err.is_parse()
            {
                return true;
            }
            let src_str = format!("{}", err).to_lowercase();
            if src_str.contains("tls")
                || src_str.contains("certificate")
                || src_str.contains("handshake")
                || src_str.contains("invaliddata")
                || src_str.contains("invalidcontenttype")
            {
                return true;
            }
            source = err.source();
        }

        false
    }
}

async fn health_check(app_state: State<'_, AppState>) -> Result<HealthStatus, String> {
    match app_state.client.read().await.get_degree().await {
        Ok(_) => Ok(HealthStatus::Ok),
        Err(ecnu_power_usage::Error::CS(CSError::EcnuNotLogin)) => Ok(HealthStatus::NotLogin),
        Err(ecnu_power_usage::Error::CS(CSError::RoomConfigMissing)) => Ok(HealthStatus::NoRoom),
        Err(ecnu_power_usage::Error::Reqwest(e)) => {
            error!(target: "health check reqwest", "{e:?}");
            if online::check(Some(Duration::from_secs(1))).await {
                if e.is_tls_error() {
                    Ok(HealthStatus::TlsError)
                } else {
                    Ok(HealthStatus::ServerDown)
                }
            } else {
                Ok(HealthStatus::NoNet)
            }
        }
        Err(e) => {
            error!(target: "error checking", "{e:?}");
            Err(e.to_string())
        }
    }
}

pub(crate) async fn health_check_routine(handle: tauri::AppHandle) -> ! {
    let mut health = HealthStatus::Ok;
    let mut network_err_count = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let state = handle.state::<AppState>();
        let check = health_check(state.clone()).await;
        *state.health.write().await = check.clone();
        let new_health = match &check {
            Ok(h) => *h,
            Err(_) => HealthStatus::Unknown,
        };
        if new_health != health {
            info!(target: "health", "new: {new_health:?}");
            if matches!(
                (health, new_health),
                (HealthStatus::Ok, HealthStatus::NoNet)
                    | (HealthStatus::Ok, HealthStatus::ServerDown)
            ) {
                if network_err_count < NETWORK_TOLERANCE_TIMES {
                    network_err_count += 1;
                    continue;
                }
            } else {
                network_err_count = 0;
            }

            info!("health status changed {health:?} -> {new_health:?}.");
            if new_health != HealthStatus::Ok {
                let (title, message) = get_notify_content(check);
                sys_notify(handle.clone(), title, message).ok(); // 内部有错误输出.
            }
            health = new_health;
        }
    }
}
