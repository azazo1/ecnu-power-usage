use std::time::Duration;

const ADDRS: &[&str] = &[
    "https://www.baidu.com",
    "https://google.com",
    "https://www.mozilla.org",
    "https://www.ecnu.edu.cn",
];

// 如果能正常连接, 返回 true
async fn connect_timeout(addr: &str, dur: Duration, client: &reqwest::Client) -> bool {
    let req = client.get(addr).send();
    match tokio::time::timeout(dur, req).await {
        Ok(resp) => resp.is_ok_and(|resp| resp.error_for_status().is_ok()),
        Err(_) => false,
    }
}

async fn connect(addr: &str, client: &reqwest::Client) -> bool {
    let req = client.get(addr).send().await;
    req.is_ok_and(|resp| resp.error_for_status().is_ok())
}

/// Check online connectivity.
pub async fn check(timeout: Option<Duration>) -> bool {
    // Avoiding `io:timeout` in this case to allow the OS decide for
    // better diagnostics.
    let client = reqwest::Client::default();
    let mut online = false;
    if let Some(dur) = timeout {
        // First try, ignoring error (if any).
        for addr in ADDRS {
            if connect_timeout(addr, dur, &client).await {
                online = true;
                break;
            }
        }
    } else {
        for addr in ADDRS {
            if connect(addr, &client).await {
                online = true;
                break;
            }
        }
    }
    online
}
