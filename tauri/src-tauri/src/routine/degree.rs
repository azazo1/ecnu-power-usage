use std::time::{Duration, Instant};

use tauri::Manager;
use tracing::{error, info};

use crate::{commands::sys_notify, config::AppState};

const NOTIFY_INTERVAL: Duration = Duration::from_mins(30);

pub(crate) async fn degree_check_routine(handle: tauri::AppHandle) -> ! {
    let mut last_notify_time: Option<Instant> = None;
    let mut last_degree: Option<f32> = None;

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        let state = handle.state::<AppState>();
        let threshold = state.config.read().await.degree_threshold();
        match state.client.read().await.get_degree().await {
            Ok(degree) => {
                if degree < threshold {
                    if let Some(last_notify_time) = last_notify_time
                        && Instant::now() - last_notify_time < NOTIFY_INTERVAL
                    {
                        continue;
                    }

                    let title = "宿舍电量不足".to_string();
                    let message = format!(
                        "当前剩余电量: {:.2} 度\n已低于设定阈值 {:.2} 度, 请及时充值.",
                        degree, threshold
                    );

                    if let Err(e) = sys_notify(handle.clone(), title, message) {
                        error!(target: "degree notification", "failed to send notification: {e}");
                    } else {
                        info!(target: "degree notification", "low degree notification sent: {} < {}", degree, threshold);
                    }

                    last_notify_time = Some(Instant::now());
                } else {
                    last_notify_time = None;
                }

                if let Some(last_degree_) = last_degree
                    && degree > last_degree_
                {
                    if let Err(e) = sys_notify(
                        handle.clone(),
                        "宿舍电量增加".to_string(),
                        format!(
                            "宿舍电量增加: {:.2} 度, 现有 {:.2} 度",
                            degree - last_degree_,
                            degree
                        ),
                    ) {
                        error!(target: "degree notification", "failed to send notification: {e}");
                    } else {
                        info!(target: "degree notification", "degree increasing notification sent: {:.2} -> {:.2}", last_degree_, degree);
                    };
                }
                last_degree = Some(degree);
            }
            Err(e) => {
                // 获取电量失败时记录错误，但不发送通知（避免干扰用户）
                error!(target: "degree check", "failed to get degree: {e:?}");
            }
        }
    }
}
