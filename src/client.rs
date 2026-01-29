use std::{io::Cursor, path::Path, time::Duration};

use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use reqwest::{Certificate, Identity, StatusCode, Url};
use tokio::{fs, task::JoinHandle};
use tracing::{error, info, warn};

use crate::{
    Cookies, Records, TimeSpan,
    config::RoomConfig,
    error::{CSError, CSResult, Error},
    server::{ArchiveMeta, CreateArchiveArgs, DeleteArchiveArgs, DownloadArchiveArgs},
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

    /// 执行, 并伴随浏览器的自动关闭.
    pub async fn with<T>(
        mut self,
        cb: impl AsyncFnOnce(&mut Self) -> crate::Result<T>,
    ) -> crate::Result<T> {
        let result = cb(&mut self).await;
        self.close().await;
        result
    }

    pub async fn launch(config: BrowserConfig) -> crate::Result<Self> {
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

    async fn wait_for_login(page: &Page) -> crate::Result<()> {
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
    pub async fn login_cookies(&self) -> crate::Result<Cookies> {
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
            return Err(Error::Cookie("JSESSIONID not found".to_string()));
        };
        let Some(cookie) = cookie else {
            return Err(Error::Cookie(
                "cookie (inside cookies) not found".to_string(),
            ));
        };

        let meta = page.find_xpath("/html/head/meta[4]").await?;
        let x_csrf_token = meta
            .property("content")
            .await?
            .ok_or_else(|| {
                Error::BrowserPage("x_csrf_token correspondant element not found".to_string())
            })?
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| {
                Error::BrowserPage("x_csrf_token element content type is not string".to_string())
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
    pub async fn pick_room(&self) -> crate::Result<RoomConfig> {
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
            .ok_or_else(|| Error::BrowserPage("#elcbuis not found".to_string()))?
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| Error::BrowserPage("#elcbuis value type is not str".to_string()))?;
        info!("elcbuis: {elcbuis:?}");

        let elcarea = page
            .find_element("#elcarea")
            .await?
            .property("value")
            .await?
            .ok_or_else(|| Error::BrowserPage("#elcarea not found".to_string()))?
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| Error::BrowserPage("#elcarea value type is not str".to_string()))?;
        info!("elcarea: {elcarea:?}");

        let room_no = page
            .find_element("#elcroom")
            .await?
            .property("value")
            .await?
            .ok_or_else(|| Error::BrowserPage("#elcroom not found".to_string()))?
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| Error::BrowserPage("#elcroom value type is not tr".to_string()))?;
        info!("elcroom(roomNo): {room_no:?}");

        page.close().await?;

        Ok(RoomConfig {
            room_no,
            elcarea: elcarea.parse().map_err(|_| {
                Error::BrowserPage("#elcarea can not be converted to int".to_string())
            })?,
            elcbuis,
        })
    }
}

pub struct Client {
    /// 服务端地址 e.g. https://localhost:20531
    server_base: Url,
    client: reqwest::Client,
}

impl Client {
    #[must_use]
    pub fn new(server_base: Url) -> Self {
        Self {
            server_base,
            client: reqwest::Client::default(),
        }
    }

    pub async fn get_degree(&self) -> crate::Result<f32> {
        let resp = self
            .client
            .get(self.server_base.join("/get-degree")?)
            .send()
            .await?;
        let result: CSResult<f32> = resp.json().await?;
        Ok(result?)
    }

    pub async fn post_cookies(&self, cookies: &Cookies) -> crate::Result<()> {
        self.client
            .post(self.server_base.join("/post-cookies")?)
            .json(cookies)
            .send()
            .await?
            // 这个请求一般不会产生错误
            .error_for_status()?;
        Ok(())
    }

    pub async fn post_room(&self, room_config: &RoomConfig) -> crate::Result<()> {
        let resp = self
            .client
            .post(self.server_base.join("/post-room")?)
            .json(room_config)
            .send()
            .await?;
        let result: CSResult<()> = resp.json().await?;
        Ok(result?)
    }

    pub async fn get_records(&self) -> crate::Result<Records> {
        let resp = self
            .client
            .get(self.server_base.join("/get-records")?)
            .send()
            .await?;
        let resp: CSResult<Records> = resp.json().await?;
        Ok(resp?)
    }

    pub async fn download_archive(&self, name: impl AsRef<str>) -> crate::Result<Records> {
        let resp = self
            .client
            .get(self.server_base.join("/download-archive")?)
            .query(&DownloadArchiveArgs {
                name: name.as_ref().to_string(),
            })
            .send()
            .await?;
        match resp.status() {
            StatusCode::OK => Ok(Records::from_csv(Cursor::new(resp.text().await?)).await?),
            _ => Err(Error::CS(resp.json().await?)),
        }
    }

    pub async fn create_archive(
        &self,
        archive_name: Option<String>,
        time_span: TimeSpan,
    ) -> crate::Result<ArchiveMeta> {
        let resp = self
            .client
            .post(self.server_base.join("/create-archive")?)
            .json(&CreateArchiveArgs {
                archive_name,
                time_span,
            })
            .send()
            .await?;
        let result: CSResult<ArchiveMeta> = resp.json().await?;
        Ok(result?)
    }

    pub async fn list_archives(&self) -> crate::Result<Vec<ArchiveMeta>> {
        let resp = self
            .client
            .get(self.server_base.join("/list-archives")?)
            .send()
            .await?;
        let result: CSResult<Vec<ArchiveMeta>> = resp.json().await?;
        Ok(result?)
    }

    pub async fn delete_archive(&self, name: impl AsRef<str>) -> crate::Result<()> {
        let resp = self
            .client
            .post(self.server_base.join("/delete-archive")?)
            .form(&DeleteArchiveArgs {
                name: name.as_ref().to_string(),
            })
            .send()
            .await?;
        let result: CSResult<()> = resp.json().await?;
        Ok(result?)
    }

    pub async fn clear_cookies(&self) -> crate::Result<()> {
        let resp = self
            .client
            .post(self.server_base.join("/clear-cookies")?)
            .send()
            .await?;
        resp.error_for_status()?;
        Ok(())
    }

    pub async fn clear_room(&self) -> crate::Result<()> {
        let resp = self
            .client
            .post(self.server_base.join("/clear-room")?)
            .send()
            .await?;
        resp.error_for_status()?;
        Ok(())
    }

    pub fn set_server_base(&mut self, server_base: Url) {
        self.server_base = server_base;
    }

    // 启用自签名 tls 证书
    pub async fn configure_tls(
        &mut self,
        client_cert: impl AsRef<Path>,
        client_key: impl AsRef<Path>,
        root_ca: impl AsRef<Path>,
    ) -> crate::Result<()> {
        let (client_cert, client_key, root_ca) =
            (client_cert.as_ref(), client_key.as_ref(), root_ca.as_ref());
        let client_cert = fs::read_to_string(client_cert).await?;
        let client_key = fs::read_to_string(client_key).await?;
        let root_ca = fs::read_to_string(root_ca).await?;
        let ident = Identity::from_pem(format!("{client_cert}\n{client_key}").as_ref())?;
        let ca = Certificate::from_pem(root_ca.as_ref())?;
        self.client = reqwest::ClientBuilder::new()
            .tls_certs_only([ca])
            .identity(ident)
            .use_rustls_tls()
            .build()?;
        Ok(())
    }

    // 关闭 tls
    pub fn deconfigure_tls(&mut self) {
        self.client = reqwest::Client::default();
    }
}

/// used for early age dev
pub struct GuardClient {
    client: Client,
    browser_config: BrowserConfig,
}

impl GuardClient {
    #[must_use]
    pub fn new(server_base: Url) -> Self {
        Self {
            client: Client::new(server_base),
            browser_config: BrowserConfig::builder().with_head().build().unwrap(),
        }
    }

    async fn with_browser<T>(
        &self,
        cb: impl AsyncFnOnce(&mut BrowserExecutor) -> crate::Result<T>,
    ) -> crate::Result<T> {
        BrowserExecutor::launch(self.browser_config.clone())
            .await?
            .with(cb)
            .await
    }

    /// 守护服务端, 保持登录状态.
    pub async fn guard(self) -> crate::Result<()> {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            match self.client.get_degree().await {
                Ok(degree) => info!("degree: {degree}"),
                Err(Error::CS(CSError::EcnuNotLogin)) => {
                    let cookies = match self.with_browser(async |be| be.login_cookies().await).await
                    {
                        Ok(cookies) => cookies,
                        Err(e) => {
                            error!("getting login cookies: {e:?}");
                            continue;
                        }
                    };

                    match self.client.post_cookies(&cookies).await {
                        Ok(()) => {
                            info!("cookies posted");
                        }
                        Err(e) => {
                            error!("cookies posting: {e:?}");
                        }
                    }
                }
                Err(Error::CS(CSError::RoomConfigMissing)) => {
                    let room_config = match self.with_browser(async |be| be.pick_room().await).await
                    {
                        Ok(room_config) => room_config,
                        Err(e) => {
                            error!("picking room: {e:?}");
                            continue;
                        }
                    };
                    match self.client.post_room(&room_config).await {
                        Ok(()) => {
                            info!("room posted");
                        }
                        Err(e) => {
                            error!("room posting: {e:?}");
                        }
                    }
                }
                Err(e) => {
                    error!("getting degree: {e:?}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        CSError, Error,
        client::{Client, GuardClient},
    };

    #[tokio::test]
    async fn list_archives() {
        let client = Client::new("http://localhost:20531".parse().unwrap());
        dbg!(client.list_archives().await).unwrap();
    }

    #[tokio::test]
    async fn download_archive() {
        let client = Client::new("http://localhost:20531".parse().unwrap());
        assert!(matches!(
            client.download_archive("not-exists").await,
            Err(Error::CS(CSError::ArchiveNotFound))
        ));
    }

    #[tokio::test]
    async fn guarding() {
        tracing_subscriber::fmt().init();

        let client = GuardClient::new("http://localhost:20531".parse().unwrap());
        client.guard().await.unwrap();
    }
}
