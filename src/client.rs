use std::time::Duration;

use chromiumoxide::{
    Browser, BrowserConfig, Page,
    cdp::browser_protocol::target::{CreateTargetParams, CreateTargetParamsBuilder},
};
use futures::StreamExt;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::{
    config::RoomConfig,
    error::{Error, Result},
};

pub struct BrowserExecutor {
    browser: Browser,
    drop_handle: JoinHandle<()>,
    closed: bool,
}

impl Drop for BrowserExecutor {
    fn drop(&mut self) {
        if !self.closed {
            warn!("BrowserExecutor was dropped before closing.");
        }
    }
}

impl BrowserExecutor {
    pub const QUERY_BILL_URL: &str =
        "https://epay.ecnu.edu.cn/epaycas/electric/load4electricbill?elcsysid=1";

    pub async fn close(mut self) -> Result<()> {
        self.drop_handle.abort();
        self.browser.close().await?;
        self.closed = true;
        Ok(())
    }

    pub async fn new(config: BrowserConfig) -> Result<Self> {
        let (browser, mut handler) = Browser::launch(config).await?;
        let drop_handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        #[cfg(debug_assertions)]
        browser.clear_cookies().await?;

        Ok(Self {
            browser,
            drop_handle,
            closed: false,
        })
    }

    async fn wait_for_login(page: &Page) -> Result<()> {
        info!("ecnu checking login state...");
        page.wait_for_navigation().await?;
        while let Some(url) = page.url().await?
            && url.starts_with("https://sso.ecnu.edu.cn")
        {
            tokio::time::sleep(Duration::from_millis(300)).await;
            page.wait_for_navigation().await?;
        }
        info!("ecnu is logined.");
        Ok(())
    }

    pub async fn pick_room(&self) -> Result<RoomConfig> {
        let page = self
            .browser
            .new_page(
                CreateTargetParams::builder()
                    .url(Self::QUERY_BILL_URL)
                    .build()
                    .map_err(Error::ChromiumParamBuildingError)?,
            )
            .await?;
        Self::wait_for_login(&page).await?;

        page.evaluate(
            r##"() => {
                let button = document.querySelector("#queryBill");
                button.onclick = function() {
                    let a = document.createElement("a");
                    a.id = "query_clicked";
                    document.body.appendChild(a);
                }
            }"##, // 查询按钮按下时添加新元素, 并在下面的代码检测这个元素.
        )
        .await?;
        info!(
            "waiting for dorm room selecting (please manually select room and click the query button)..."
        );

        // 等待元素出现
        while let Err(e) = page.find_element("#query_clicked").await {
            tokio::time::sleep(Duration::from_millis(100)).await;
            match e {
                // 这两个分支暂时不清楚是干什么的, 先跳过.
                chromiumoxide::error::CdpError::Timeout
                | chromiumoxide::error::CdpError::NotFound => continue,
                // -32000: Could not find node with given id
                chromiumoxide::error::CdpError::Chrome(ce) if ce.code == -32000 => continue,
                _ => Err(dbg!(e))?,
            }
        }

        let elcbuis = page
            .find_element("#elcbuis")
            .await?
            .property("value")
            .await?
            .ok_or_else(|| Error::BrowserPageError("#elcbuis not found".to_string()))?
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| Error::BrowserPageError("#elcbuis value type is not str".to_string()))?;
        info!("elcbuis: {elcbuis:?}");

        let elcarea = page
            .find_element("#elcarea")
            .await?
            .property("value")
            .await?
            .ok_or_else(|| Error::BrowserPageError("#elcarea not found".to_string()))?
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| Error::BrowserPageError("#elcarea value type is not str".to_string()))?;
        info!("elcarea: {elcarea:?}");

        let room_no = page
            .find_element("#elcroom")
            .await?
            .property("value")
            .await?
            .ok_or_else(|| Error::BrowserPageError("#elcroom not found".to_string()))?
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| Error::BrowserPageError("#elcroom value type is not tr".to_string()))?;
        info!("elcroom(roomNo): {room_no:?}");

        page.close().await?;

        Ok(RoomConfig {
            room_no,
            elcarea: elcarea.parse().map_err(|_| {
                Error::BrowserPageError("#elcarea can not be converted to int".to_string())
            })?,
            elcbuis,
        })
    }
}
