use std::time::Duration;

use futures_util::StreamExt;

const ADDRS: [&str; 4] = [
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
    let mut online = false;
    // 非常奇怪的生命周期问题,我这里如果不对 ADDR 执行 to_string, 就会报错,
    // 而且不是在这里报错, 而是在 tauri 调用的位置报错.
    let mut stream = futures_util::stream::iter(ADDRS.map(str::to_string))
        .map(|addr| async move {
            let client = reqwest::Client::default();
            if let Some(dur) = timeout {
                connect_timeout(&addr, dur, &client).await
            } else {
                connect(&addr, &client).await
            }
        })
        .buffer_unordered(3);
    while let Some(c) = stream.next().await {
        if c {
            online = true;
            break;
        }
    }
    online
}
