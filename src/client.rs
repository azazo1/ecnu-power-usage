use std::time::Duration;

use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use reqwest::{Client, Url};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::{
    Cookies,
    config::RoomConfig,
    error::{Error, Result},
    server::GetDegreeResponse,
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

    pub async fn close(mut self) {
        self.drop_handle.abort();
        self.closed = true;
        match self.browser.close().await {
            Ok(_) => (),
            Err(_) => {
                self.browser.kill().await;
            }
        }
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

    /// 使用浏览器交互式进行 ECNU 登录, 获取登录 cookies, 用于上传给服务器.
    pub async fn login_cookies(&self) -> Result<Cookies> {
        let page = self.browser.new_page(Self::QUERY_BILL_URL).await?;
        Self::wait_for_login(&page).await?;
        let cookies = self.browser.get_cookies().await?;
        let mut j_session_id = None;
        let mut cookie = None;

        for ck in cookies {
            match ck.name.as_str() {
                "JSESSIONID" => j_session_id = Some(ck.value),
                "cookie" => cookie = Some(ck.value),
                _ => (),
            }
        }

        let Some(j_session_id) = j_session_id else {
            return Err(Error::CookieError("JSESSIONID not found".to_string()));
        };
        let Some(cookie) = cookie else {
            return Err(Error::CookieError(
                "cookie (inside cookies) not found".to_string(),
            ));
        };

        let meta = page.find_xpath("/html/head/meta[4]").await?;
        let x_csrf_token = meta
            .property("content")
            .await?
            .ok_or_else(|| {
                Error::BrowserPageError("x_csrf_token correspondant element not found".to_string())
            })?
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| {
                Error::BrowserPageError(
                    "x_csrf_token element content type is not string".to_string(),
                )
            })?;

        page.close().await.ok();

        Ok(Cookies {
            j_session_id,
            cookie,
            x_csrf_token,
        })
    }

    /// 使用浏览器在网页交互式地选择宿舍信息.
    ///
    /// # Errors
    /// - [`Error::ChromiumParamBuildingError`][]: 正常情况不应出现.
    /// - [`Error::BrowserPageError`][]: 选择页面被改造了, 此套代码需要更新以适配.
    /// - [`Error::ChromiumError`][]: see: [`Browser::new_page`], [`Self::wait_for_login`],
    ///   [`Page::evaluate`], [`Page::find_element`].
    pub async fn pick_room(&self) -> Result<RoomConfig> {
        let page = self.browser.new_page(Self::QUERY_BILL_URL).await?;
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
                _ => Err(e)?,
            }
        }

        let elcbuis = page
            .find_element("#elcbuis")
            .await?
            .property("value")
            .await?
            .ok_or_else(|| Error::BrowserPageError("#elcbuis not found".to_string()))?
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| Error::BrowserPageError("#elcbuis value type is not str".to_string()))?;
        info!("elcbuis: {elcbuis:?}");

        let elcarea = page
            .find_element("#elcarea")
            .await?
            .property("value")
            .await?
            .ok_or_else(|| Error::BrowserPageError("#elcarea not found".to_string()))?
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| Error::BrowserPageError("#elcarea value type is not str".to_string()))?;
        info!("elcarea: {elcarea:?}");

        let room_no = page
            .find_element("#elcroom")
            .await?
            .property("value")
            .await?
            .ok_or_else(|| Error::BrowserPageError("#elcroom not found".to_string()))?
            .as_str()
            .map(str::to_string)
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

pub struct GuardClient {
    /// 服务端地址 e.g. https://localhost:20531
    server_base: Url,
    client: Client,
    browser_config: BrowserConfig,
}

impl GuardClient {
    // todo tls
    #[must_use]
    pub fn new(server_host: Url) -> Self {
        Self {
            server_base: server_host,
            client: Client::default(),
            browser_config: BrowserConfig::builder().with_head().build().unwrap(),
        }
    }

    async fn with_browser<T>(
        &self,
        cb: impl AsyncFnOnce(&mut BrowserExecutor) -> Result<T>,
    ) -> Result<T> {
        let mut be = BrowserExecutor::new(self.browser_config.clone()).await?;
        let rst = cb(&mut be).await;
        be.close().await;
        rst
    }

    /// 守护服务端, 保持登录状态.
    pub async fn guard(self) -> Result<()> {
        // todo: 下面所有的失败 GUI 信息都可以提供延迟提醒.
        // todo: 重试机制.
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            match self.get_degree().await {
                Ok(resp) => match resp {
                    GetDegreeResponse::Logined(degree) => info!("degree: {degree}"),
                    GetDegreeResponse::NotLogined => {
                        let cookies =
                            match self.with_browser(async |be| be.login_cookies().await).await {
                                Ok(cookies) => cookies,
                                Err(e) => {
                                    error!("getting login cookies: {e:?}");
                                    // todo!("GUI 提示获取 cookies 失败.");
                                    continue;
                                }
                            };

                        match self.post_cookies(&cookies).await {
                            Ok(()) => {
                                info!("cookies posted");
                                // todo!("GUI 显示成功提交 Cookies");
                            }
                            Err(e) => {
                                error!("cookies posting: {e:?}");
                                // todo!("GUI 显示提交 Cookies 失败");
                            }
                        }
                    }
                    GetDegreeResponse::RoomConfigMissing => {
                        // todo!("GUI 提示需要进行选择宿舍房间信息")
                        let room_config =
                            match self.with_browser(async |be| be.pick_room().await).await {
                                Ok(room_config) => room_config,
                                Err(e) => {
                                    error!("picking room: {e:?}");
                                    // todo!("GUI 提醒获取房间信息失败");
                                    continue;
                                }
                            };
                        match self.post_room(&room_config).await {
                            Ok(()) => {
                                info!("room posted");
                                // todo!("GUI 提示成功提交 room config");
                            }
                            Err(e) => {
                                error!("room posting: {e:?}");
                                // todo!("GUI 提示 room 信息提交失败");
                            }
                        }
                    }
                    GetDegreeResponse::Error(e) => {
                        error!("server getting degree: {e:?}");
                        // todo!("GUI 提醒服务端请求 degree 失败");
                    }
                },
                Err(e) => {
                    error!("getting degree: {e:?}");
                    // todo!("GUI 提醒无法连接到服务端");
                }
            }
        }
    }

    pub async fn get_degree(&self) -> Result<GetDegreeResponse> {
        let resp = self
            .client
            .get(self.server_base.join("/get-degree")?)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn post_cookies(&self, cookies: &Cookies) -> Result<()> {
        self.client
            .post(self.server_base.join("/post-cookies")?)
            .json(cookies)
            .send()
            .await?;
        Ok(())
    }

    pub async fn post_room(&self, room_config: &RoomConfig) -> Result<()> {
        self.client
            .post(self.server_base.join("/post-room")?)
            .json(room_config)
            .send()
            .await?;
        Ok(())
    }
}
