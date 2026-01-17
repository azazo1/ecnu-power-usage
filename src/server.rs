//! 服务端逻辑.

use std::borrow::Cow;

use reqwest::header::COOKIE;
use reqwest::{Client, Method};
use serde_json::json;

use crate::Cookies;
use crate::config::RoomConfig;
use crate::error::{Error, Result};

#[derive(serde::Deserialize)]
struct QueryResponse {
    #[serde(rename = "retcode")]
    code: i32,
    #[serde(rename = "retmsg")]
    msg: String,
    #[serde(rename = "restElecDegree")]
    degree: Option<f32>,
}

/// 用于指定宿舍电量查询.
pub struct Querier {
    config: RoomConfig,
    x_csrf_token: String,
    cookies: Cookies,
    client: Client,
}

impl Querier {
    pub fn new(config: RoomConfig) -> Querier {
        Querier {
            config,
            x_csrf_token: "".into(),
            cookies: Default::default(),
            client: Default::default(),
        }
    }

    pub fn new_with_client(config: RoomConfig, client: Client) -> Querier {
        Querier {
            config,
            x_csrf_token: "".into(),
            cookies: Default::default(),
            client,
        }
    }

    /// 重新设置有效的 x_csrf_token 和 cookies.
    pub fn refresh(&mut self, x_csrf_token: String, cookies: Cookies) {
        self.x_csrf_token = x_csrf_token;
        self.cookies = cookies.sanitize();
    }

    /// 查询查询当前剩余电量 (度)
    pub async fn query_electricity_balance(&self) -> Result<f32> {
        let payload = json!({
            "sysid": 1,
            "roomNo": self.config.room_no.as_str(),
            "elcarea": self.config.elcarea,
            "elcbuis": self.config.elcbuis.as_str(),
        });
        let resp = self
            .client
            .request(
                Method::POST,
                "https://epay.ecnu.edu.cn/epaycas/electric/queryelectricbill",
            )
            .header(
                COOKIE,
                format!(
                    "JSESSIONID={}; cookie={}",
                    self.cookies.j_session_id, self.cookies.cookie
                ),
            )
            .header("X-CSRF-TOKEN", &self.x_csrf_token)
            // todo 解决 cookies 登录状态问题
            .json(&payload)
            .send()
            .await?;
        if let Some(ct) = resp.headers().get("Content-Type")
            && let Ok(ct) = ct.to_str()
            && !ct.contains("application/json")
        {
            Err(Error::EcnuError("Permission Denied".to_string()))?
        }
        let ret: QueryResponse = resp.json().await?;
        if ret.code != 0 || ret.msg != "成功" {
            Err(Error::EcnuError(ret.msg))?
        }
        ret.degree.ok_or(Error::NoDegree)
    }
}
