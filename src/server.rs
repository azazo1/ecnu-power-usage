//! 服务端逻辑.
use std::ops::Sub;
use std::path::Path;

use chrono::{DateTime, Local};
use reqwest::header::COOKIE;
use reqwest::{Client, Method};
use serde_json::json;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

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

struct Recorder<W>
where
    W: AsyncWrite + Unpin,
{
    out: W,
    last_time_degree_pair: Option<(DateTime<Local>, f32)>,
}

impl<W> Recorder<W>
where
    W: AsyncWrite + Unpin,
{
    /// 尝试记录一次电费, 如果有新纪录产生到输出流, 那么返回这个记录.
    pub async fn record(&mut self, degree: f32) -> Result<Option<(DateTime<Local>, f32)>> {
        if let Some((last_time, last_degree)) = self.last_time_degree_pair {
            if last_degree.sub(degree).abs() < 0.01 {
                return Ok(None);
            }
            let line = format!("{},{}\n", last_time.to_rfc3339(), last_degree);
            self.out.write_all(line.as_bytes()).await?;
            return Ok(Some((last_time, last_degree)));
        }
        let now_time = Local::now();
        self.last_time_degree_pair = Some((now_time, degree));
        Ok(None)
    }

    #[must_use]
    pub fn new(out: W) -> Self {
        Recorder {
            out,
            last_time_degree_pair: None,
        }
    }
}

impl Recorder<File> {
    pub async fn load_records(records_path: impl AsRef<Path>) -> Result<Recorder<File>> {
        let records_path = records_path.as_ref();
        let read = OpenOptions::new().read(true).open(records_path).await?;
        let read = BufReader::new(read);
        let mut last_line = None;
        let mut lines = read.lines();
        while let Some(line) = lines.next_line().await? {
            if !line.is_empty() {
                last_line = Some(line);
            }
        }
        let last_time_degree_pair = if let Some(last_line) = last_line {
            let (time, degree) = last_line
                .split_once(',')
                .ok_or(Error::DegreeRecordsFormatError)?;
            let time = DateTime::parse_from_rfc3339(time)?.with_timezone(&Local);
            let degree: f32 = degree.trim().parse()?;
            Some((time, degree))
        } else {
            None
        };
        let write = OpenOptions::new()
            .create(false)
            .append(true)
            .open(records_path)
            .await?;
        Ok(Recorder {
            out: write,
            last_time_degree_pair,
        })
    }
}
