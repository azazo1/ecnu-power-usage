use std::{fmt::Display, time::Duration};

use serde::Serialize;
use tauri::Manager;
use tracing::info;

use crate::{
    commands::{health_check, sys_notify},
    config::AppState,
};

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

pub(crate) async fn init_health_check_routine(handle: tauri::AppHandle) -> ! {
    let mut health = HealthStatus::Ok;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let state = handle.state::<AppState>();
        let check = health_check(state).await;
        let new_health = match &check {
            Ok(h) => *h,
            Err(_) => HealthStatus::Unknown,
        };
        if new_health != health {
            info!("health status changed {health:?} -> {new_health:?}.");
            if new_health != HealthStatus::Ok {
                let (title, message) = get_notify_content(check);
                sys_notify(handle.clone(), title, message).ok(); // 内部有错误输出.
            }
            health = new_health;
        }
    }
}
