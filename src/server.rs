//! 服务端逻辑.
use std::fmt::{Debug, Display};
use std::net::SocketAddr;
use std::ops::Sub;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Local};
use reqwest::header::COOKIE;
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::config::{RECORDS_FILENAME, ROOM_CONFIG_FILENAME, RoomConfig, load_room_config};
use crate::error::{Error, Result};
use crate::{Cookies, Records};

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
#[derive(Default)]
pub struct Querier {
    config: RoomConfig,
    cookies: Cookies,
    client: Client,
}

impl Debug for Querier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Querier")
            .field("config", &self.config)
            .field("cookies", &self.cookies)
            .field("client", &self.client)
            .finish()
    }
}

impl Querier {
    #[must_use]
    pub fn new(config: RoomConfig) -> Querier {
        Querier {
            config,
            cookies: Default::default(),
            client: Default::default(),
        }
    }

    #[must_use]
    pub fn new_with_client(config: RoomConfig, client: Client) -> Querier {
        Querier {
            config,
            cookies: Default::default(),
            client,
        }
    }

    pub fn set_room_config(&mut self, config: RoomConfig) {
        self.config = config;
    }

    /// 重新设置有效的 cookies.
    pub fn refresh(&mut self, cookies: Cookies) {
        self.cookies = cookies.sanitize();
    }

    /// 查询查询当前剩余电量 (度)
    /// # Errors
    /// - [`Error::ReqwestError`][]: see: [`reqwest::RequestBuilder::send`].
    /// - [`Error::EcnuError`][]: ECNU 未登录 / 查询接口返回错误信息.
    /// - [`Error::NoDegree`][]: 不应出现此情况, 如果出现了可能是接口返回了错误的数据.
    pub async fn query_electricity_degree(&self) -> Result<f32> {
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
            .header("X-CSRF-TOKEN", &self.cookies.x_csrf_token)
            // todo 解决 cookies 登录状态问题
            .form(&payload)
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

impl<W> Debug for Recorder<W>
where
    W: AsyncWrite + Unpin,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recorder")
            .field("out", &"...")
            .field("last_time_degree_pair", &self.last_time_degree_pair)
            .finish()
    }
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
    pub async fn load_from_records(records_path: impl AsRef<Path>) -> Result<Recorder<File>> {
        let records_path = records_path.as_ref();
        let mut last_line = None;
        if let Ok(read) = OpenOptions::new().read(true).open(records_path).await {
            let read = BufReader::new(read);
            let mut lines = read.lines();
            while let Some(line) = lines.next_line().await? {
                if !line.is_empty() {
                    last_line = Some(line);
                }
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
            .create(true)
            .append(true)
            .open(records_path)
            .await?;
        Ok(Recorder {
            out: write,
            last_time_degree_pair,
        })
    }
}

#[derive(Debug)]
struct AppState {
    querier: RwLock<Querier>,
    recorder: RwLock<Recorder<File>>,
    data_dir: PathBuf,
    config_dir: PathBuf,
}

/// post room info
async fn post_room(
    State(state): State<Arc<AppState>>,
    Json(room_config): Json<RoomConfig>,
) -> StatusCode {
    info!("post room request");
    todo!("还要写入 room.csv")
}

async fn post_cookies(
    State(state): State<Arc<AppState>>,
    Json(cookies): Json<Cookies>,
) -> StatusCode {
    info!("post cookies request");
    state.querier.write().await.refresh(cookies);
    StatusCode::OK
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetRecordsResponse {
    pub records: Option<Records>,
    pub msg: String,
}

async fn get_records(State(state): State<Arc<AppState>>) -> (StatusCode, Json<GetRecordsResponse>) {
    let records_path = state.data_dir.join(RECORDS_FILENAME);
    match csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(records_path)
    {
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GetRecordsResponse {
                records: None,
                msg: format!("records file read error: {e:?}"),
            }),
        ),
        Ok(mut rdr) => {
            let des: csv::Result<_> = rdr.deserialize::<(DateTime<Local>, f32)>().collect();
            match des {
                Ok(des) => {
                    let records = Records(des);
                    (
                        StatusCode::OK,
                        Json(GetRecordsResponse {
                            records: Some(records),
                            msg: "ok".to_string(),
                        }),
                    )
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GetRecordsResponse {
                        records: None,
                        msg: format!("invalid records file format: {e:}"),
                    }),
                ),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status", content = "content")]
pub enum GetDegreeResponse {
    Logined(f32),
    NotLogined,
    RoomConfigMissing,
    Error(String),
}

async fn get_degree(State(state): State<Arc<AppState>>) -> (StatusCode, Json<GetDegreeResponse>) {
    info!("get degree request");
    let querier = state.querier.read().await;
    if querier.config.is_invalid() {
        return (StatusCode::OK, Json(GetDegreeResponse::RoomConfigMissing));
    }
    match querier.query_electricity_degree().await {
        Ok(degree) => (StatusCode::OK, Json(GetDegreeResponse::Logined(degree))),
        Err(e) => match e {
            Error::EcnuError(_) => (StatusCode::OK, Json(GetDegreeResponse::NotLogined)),
            e => (
                StatusCode::OK,
                Json(GetDegreeResponse::Error(e.to_string())),
            ),
        },
    }
}

async fn record_loop(state: Arc<AppState>) -> ! {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        match state.querier.read().await.query_electricity_degree().await {
            Ok(degree) => {
                info!("degree: {degree:.2}");
                if let Err(e) = state.recorder.write().await.record(degree).await {
                    error!("recording: {e:?}");
                }
            }
            Err(e) => {
                error!("querying: {e:?}");
            }
        }
    }
}

/// todo: tls 支持
/// todo: archive records
/// 创建并启动后台服务.
pub async fn run_app(bind_address: SocketAddr) -> anyhow::Result<()> {
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    let backup_data_dir = shellexpand::tilde("~/.local/share");
    let backup_config_dir = shellexpand::tilde("~/.config");
    let data_dir = dirs_next::data_dir()
        .unwrap_or(backup_data_dir.to_string().into())
        .join(PKG_NAME);
    let config_dir = dirs_next::config_dir()
        .unwrap_or(backup_config_dir.to_string().into())
        .join(PKG_NAME);
    info!("data dir: {data_dir:?}");
    info!("config dir: {config_dir:?}");
    let data_dir_cloned = data_dir.clone();
    let config_dir_cloned = config_dir.clone();
    fs::create_dir_all(&data_dir)
        .await
        .with_context(move || data_dir_cloned.to_string_lossy().to_string())?;
    fs::create_dir_all(&config_dir)
        .await
        .with_context(move || config_dir_cloned.to_string_lossy().to_string())?;
    let data_dir_cloned = data_dir.clone();
    let config_dir_cloned = config_dir.clone();

    let room_config = load_room_config(config_dir.join(ROOM_CONFIG_FILENAME))
        .await
        .with_context(move || {
            config_dir_cloned
                .join(ROOM_CONFIG_FILENAME)
                .to_string_lossy()
                .to_string()
        });
    info!("room config: {room_config:#?}");
    let app_state = Arc::new(AppState {
        querier: RwLock::new(if let Ok(room_config) = room_config {
            Querier::new(room_config)
        } else {
            Querier::default()
        }),
        recorder: RwLock::new(
            Recorder::load_from_records(data_dir.join(RECORDS_FILENAME))
                .await
                .with_context(move || {
                    data_dir_cloned
                        .join(RECORDS_FILENAME)
                        .to_string_lossy()
                        .to_string()
                })?,
        ),
        data_dir,
        config_dir,
    });
    let router = Router::new()
        .route("/post-room", post(post_room))
        .route("/post-cookies", post(post_cookies))
        .route("/get-records", get(get_records))
        .route("/get-degree", get(get_degree))
        .with_state(Arc::clone(&app_state));
    let listener = TcpListener::bind(bind_address).await?;
    let handle = tokio::spawn(async move { record_loop(app_state).await });
    axum::serve(listener, router).await?;
    handle.await?;
    Ok(())
}
