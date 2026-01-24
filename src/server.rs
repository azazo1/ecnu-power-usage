//! 服务端逻辑.
use std::fmt::Debug;
use std::io::SeekFrom;
use std::net::SocketAddr;
use std::ops::Sub;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderName, Response};
use axum::http::{StatusCode, header::COOKIE};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use chrono::{DateTime, FixedOffset, Local, TimeZone};
use reqwest::header::CONTENT_TYPE;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs::{self, File};
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use tracing::{debug, error, info};

use crate::config::{ARCHIVE_DIRNAME, RECORDS_FILENAME, ROOM_CONFIG_FILENAME, RoomConfig};
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
    /// 最后一个记录的电量, 保证已经被输出到 out 之中.
    last_degree: Option<f32>,
}

impl<W> Debug for Recorder<W>
where
    W: AsyncWrite + Unpin,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recorder")
            .field("out", &"...")
            .field("last_degree", &self.last_degree)
            .finish()
    }
}

impl<W> Recorder<W>
where
    W: AsyncWrite + Unpin,
{
    /// 写入一个时间, 剩余度数对到输出中.
    async fn record_instant(&mut self, time: DateTime<FixedOffset>, degree: f32) -> Result<()> {
        let line = format!("{},{}\n", time.to_rfc3339(), degree);
        self.out.write_all(line.as_bytes()).await?;
        self.last_degree = Some(degree);
        Ok(())
    }

    async fn record_multiple(&mut self, records: &Records) -> Result<()> {
        for rec in records.iter() {
            self.record_instant(rec.0, rec.1).await?;
        }
        Ok(())
    }

    /// 尝试记录一次电量变化, 只有产生了电量度数的变化才会被记录, 如果被记录了, 那么返回 Ok(true).
    pub async fn record(&mut self, degree: f32) -> Result<bool> {
        let now_time = Local::now().fixed_offset();
        if let Some(last_degree) = self.last_degree
            && last_degree.sub(degree).abs() < 0.01
        {
            return Ok(false);
        }

        self.record_instant(now_time, degree).await?;
        Ok(true)
    }
}

impl Recorder<File> {
    /// 从可读可写文件中加载.
    pub async fn load_from_rw_file(mut file: File) -> Result<Recorder<File>> {
        file.seek(SeekFrom::Start(0)).await?;

        let mut last_line = None;

        let mut lines = BufReader::new(&mut file).lines();
        while let Some(line) = lines.next_line().await? {
            if !line.is_empty() {
                last_line = Some(line);
            }
        }

        let last_degree = if let Some(last_line) = last_line {
            let (_, degree) = last_line
                .split_once(',')
                .ok_or(Error::DegreeRecordsFormatError)?;
            let degree: f32 = degree.trim().parse()?;
            Some(degree)
        } else {
            None
        };
        Ok(Recorder {
            out: file,
            last_degree,
        })
    }

    /// 从路径中加载, 如果文件不存在, 文件将被创建, 并返回对应没有任何记录 Recorder.
    pub async fn load_from_path(records_path: impl AsRef<Path>) -> Result<Recorder<File>> {
        let records_path = records_path.as_ref();
        let file = File::options()
            .read(true)
            .write(true)
            .append(false)
            .truncate(false)
            .create(true)
            .open(records_path)
            .await?;
        Self::load_from_rw_file(file).await
    }

    /// 将符合时间范围的记录摘取出来, 从 records.csv 中去除.
    ///
    /// # Errors
    ///
    /// - 需要 File 输出对象是 Seekable 和 Readable 的, 不然将会返回 [`Error::IoError`].
    pub async fn archive(&mut self, time_range: TimeSpan) -> Result<Records> {
        self.out.seek(SeekFrom::Start(0)).await?;
        let records = Records::from_csv(&mut self.out).await?;
        let mut archived = Vec::new();
        let mut retained = Vec::new();
        for rec in records.0 {
            if time_range.contains(&rec.0) {
                archived.push(rec);
            } else {
                retained.push(rec);
            }
        }
        self.out.set_len(0).await?;
        self.out.seek(SeekFrom::Start(0)).await?;
        for rec in retained {
            self.record_instant(rec.0, rec.1).await?;
        }
        self.out.seek(SeekFrom::End(0)).await?;
        Ok(Records(archived))
    }

    /// 从文件中读取已经输出的 records.
    pub async fn read_records(&mut self) -> Result<Records> {
        self.out.seek(SeekFrom::Start(0)).await?;
        let rst = Records::from_csv(&mut self.out).await;
        self.out.seek(SeekFrom::End(0)).await?;
        rst
    }
}

#[derive(Debug)]
struct AppState {
    querier: RwLock<Querier>,
    recorder: RwLock<Recorder<File>>,
    data_dir: PathBuf,
    config_dir: PathBuf,
}

#[derive(Deserialize, Serialize)]
pub enum PostRoomResponse {
    Success,
    FailedToSave,
}

async fn post_room(
    State(state): State<Arc<AppState>>,
    Json(room_config): Json<RoomConfig>,
) -> (StatusCode, Json<PostRoomResponse>) {
    info!("post room request");
    state.querier.write().await.config = room_config.clone();
    if let Err(e) = room_config
        .save_to_file(state.config_dir.join(ROOM_CONFIG_FILENAME))
        .await
    {
        error!("saving room config: {e:?}");
        (StatusCode::OK, Json(PostRoomResponse::FailedToSave))
    } else {
        (StatusCode::OK, Json(PostRoomResponse::Success))
    }
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
#[serde(tag = "tag", content = "content")]
pub enum GetRecordsResponse {
    Ok(Records),
    Error(String),
}

async fn get_records(State(state): State<Arc<AppState>>) -> (StatusCode, Json<GetRecordsResponse>) {
    match state.recorder.write().await.read_records().await {
        Ok(records) => (StatusCode::OK, Json(GetRecordsResponse::Ok(records))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GetRecordsResponse::Error(e.to_string())),
        ),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "tag", content = "content")]
pub enum GetDegreeResponse {
    Logined(f32),
    NotLogined,
    RoomConfigMissing,
    Error(String),
}

async fn get_degree(State(state): State<Arc<AppState>>) -> (StatusCode, Json<GetDegreeResponse>) {
    debug!("get degree request");
    let querier = state.querier.read().await;
    if querier.config.is_invalid() {
        return (StatusCode::OK, Json(GetDegreeResponse::RoomConfigMissing));
    }
    match querier.query_electricity_degree().await {
        Ok(degree) => (StatusCode::OK, Json(GetDegreeResponse::Logined(degree))),
        Err(e) => match e {
            Error::EcnuError(_) => (StatusCode::OK, Json(GetDegreeResponse::NotLogined)),
            e => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetDegreeResponse::Error(e.to_string())),
            ),
        },
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TimeSpan {
    /// 时间范围: 开头 (包含)
    pub start_time: Option<DateTime<FixedOffset>>,
    /// 时间范围: 末尾 (不包含)
    pub end_time: Option<DateTime<FixedOffset>>,
}

impl TimeSpan {
    pub const ALL: Self = TimeSpan::new(None, None);

    #[must_use]
    pub const fn new(
        start_time: Option<DateTime<FixedOffset>>,
        end_time: Option<DateTime<FixedOffset>>,
    ) -> Self {
        Self {
            start_time,
            end_time,
        }
    }

    #[must_use]
    pub const fn new_before(end_time: DateTime<FixedOffset>) -> Self {
        Self::new(None, Some(end_time))
    }

    #[must_use]
    pub const fn new_after(start_time: DateTime<FixedOffset>) -> Self {
        Self::new(Some(start_time), None)
    }

    #[must_use]
    pub fn contains<Tz: TimeZone>(&self, o: &DateTime<Tz>) -> bool {
        self.start_time.is_none_or(|st| st.le(o)) && self.end_time.is_none_or(|et| et.gt(o))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "tag", content = "content")]
pub enum CreateArchiveResponse {
    /// 保存的 archive 名.
    Saved(String),
    /// 当前记录为空.
    Empty,
    Error(String),
}

/// 创建 archive, 将符合时间范围的 records 保存到 archives 之中.
async fn create_archive(
    State(state): State<Arc<AppState>>,
    Json(time_range): Json<TimeSpan>,
) -> (StatusCode, Json<CreateArchiveResponse>) {
    let mut recorder = state.recorder.write().await;
    match recorder.archive(time_range).await {
        Ok(mut archived) => {
            archived.sort();
            if let Some((start_time, _)) = archived.time_span() {
                let archive_dir = state.data_dir.join(ARCHIVE_DIRNAME);
                let archive_name = start_time.format("%Y-%m-%d.csv").to_string();
                fs::create_dir_all(&archive_dir).await.ok(); // 如果失败了会在下面报错;
                match Recorder::load_from_path(archive_dir.join(&archive_name)).await {
                    Ok(mut recorder) => match recorder.record_multiple(&archived).await {
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(CreateArchiveResponse::Error(e.to_string())),
                        ),
                        _ => (
                            StatusCode::OK,
                            Json(CreateArchiveResponse::Saved(archive_name)),
                        ),
                    },
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(CreateArchiveResponse::Error(e.to_string())),
                    ),
                }
            } else {
                // 如果 records 无法计算出时间跨度, 那么说明其为空.
                (StatusCode::OK, Json(CreateArchiveResponse::Empty))
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateArchiveResponse::Error(e.to_string())),
        ),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DownloadArchiveArgs {
    name: String,
}

async fn download_archive(
    State(state): State<Arc<AppState>>,
    Form(args): Form<DownloadArchiveArgs>,
) -> Response<Body> {
    info!("download archive request: {}", args.name);
    let archive_dir = state.data_dir.join(ARCHIVE_DIRNAME);

    let Some(archive_name) = Path::new(&args.name).file_name() else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::new("no archive name in request".to_string()))
            .unwrap()
            .into_response();
    };

    match File::open(archive_dir.join(archive_name)).await {
        Ok(file) => {
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/csv")
                .header(
                    "Content-Disposition",
                    format!(
                        "attachment; filename=\"{}\"",
                        archive_name.to_string_lossy()
                    ),
                )
                .body(body)
                .unwrap()
                .into_response()
        }
        Err(_) => {
            // 失败：返回 404 和普通文本
            (StatusCode::NOT_FOUND, "File not found").into_response()
        }
    }
}

async fn record_loop(state: Arc<AppState>) -> ! {
    enum LoopState {
        Normal,
        NotLogined,
    }

    let mut interval = tokio::time::interval(Duration::from_secs(10));
    let mut loop_state = LoopState::Normal;
    loop {
        interval.tick().await;
        match state.querier.read().await.query_electricity_degree().await {
            Ok(degree) => {
                info!("degree: {degree:.2}");
                if let Err(e) = state.recorder.write().await.record(degree).await {
                    error!("recording: {e:?}");
                }
                loop_state = LoopState::Normal;
            }
            Err(e) => match loop_state {
                LoopState::Normal => {
                    error!("querying: {e:?}");
                    if matches!(e, Error::EcnuError(_)) {
                        loop_state = LoopState::NotLogined;
                    }
                }
                LoopState::NotLogined => {
                    if !matches!(e, Error::EcnuError(_)) {
                        error!("querying: {e:?}");
                    }
                }
            },
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

    let room_config = RoomConfig::from_file(config_dir.join(ROOM_CONFIG_FILENAME))
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
            Recorder::load_from_path(data_dir.join(RECORDS_FILENAME))
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
        .route("/create-archive", get(create_archive))
        .route("/download-archive", get(download_archive))
        .with_state(Arc::clone(&app_state));
    let listener = TcpListener::bind(bind_address).await?;
    let handle = tokio::spawn(async move { record_loop(app_state).await });
    axum::serve(listener, router).await?;
    handle.await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use chrono::{FixedOffset, TimeZone, Timelike};
    use tokio::fs::File;

    use crate::{
        Records,
        server::{Recorder, TimeSpan},
    };

    #[tokio::test]
    async fn archive() {
        let records = Records::from_csv(Cursor::new(
            "\
2026-01-24T15:39:32.132936+08:00,33.63
2026-01-24T17:06:32.132936+08:00,33.96
2026-01-24T18:33:32.132936+08:00,34.45
2026-01-24T20:09:32.132936+08:00,34.99
2026-01-24T20:30:32.132936+08:00,35.15
2026-01-24T20:48:32.132936+08:00,35.20
2026-01-24T22:23:32.132936+08:00,35.57
2026-01-24T23:25:32.132936+08:00,35.76
2026-01-25T01:22:32.132936+08:00,36.67
2026-01-25T03:13:32.132936+08:00,36.87
2026-01-25T04:49:32.132936+08:00,37.56
2026-01-25T05:10:32.132936+08:00,37.69
2026-01-25T06:45:32.132936+08:00,38.36
2026-01-25T07:59:32.132936+08:00,38.96
2026-01-25T09:48:32.132936+08:00,39.66
2026-01-25T11:31:32.132936+08:00,40.36
2026-01-25T12:02:32.132936+08:00,40.47
2026-01-25T13:56:32.132936+08:00,40.97
2026-01-25T15:54:32.132936+08:00,41.32
2026-01-25T16:09:32.132936+08:00,41.43",
        ))
        .await
        .unwrap();
        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let ts = TimeSpan::new_before(offset.with_ymd_and_hms(2026, 1, 25, 11, 30, 0).unwrap());
        let file = File::from(tempfile::tempfile().unwrap());
        let mut recorder = Recorder::load_from_rw_file(file).await.unwrap();
        recorder.record_multiple(&records).await.unwrap();
        let archived = recorder.archive(ts).await.unwrap();
        #[rustfmt::skip]
        assert_eq!(
            archived.0,
            vec![
                (offset.with_ymd_and_hms(2026, 1, 24, 15, 39, 32).unwrap().with_nanosecond(132936000).unwrap(), 33.63f32),
                (offset.with_ymd_and_hms(2026, 1, 24, 17, 6, 32).unwrap().with_nanosecond(132936000).unwrap(), 33.96f32),
                (offset.with_ymd_and_hms(2026, 1, 24, 18, 33, 32).unwrap().with_nanosecond(132936000).unwrap(), 34.45f32),
                (offset.with_ymd_and_hms(2026, 1, 24, 20, 9, 32).unwrap().with_nanosecond(132936000).unwrap(), 34.99f32),
                (offset.with_ymd_and_hms(2026, 1, 24, 20, 30, 32).unwrap().with_nanosecond(132936000).unwrap(), 35.15f32),
                (offset.with_ymd_and_hms(2026, 1, 24, 20, 48, 32).unwrap().with_nanosecond(132936000).unwrap(), 35.20f32),
                (offset.with_ymd_and_hms(2026, 1, 24, 22, 23, 32).unwrap().with_nanosecond(132936000).unwrap(), 35.57f32),
                (offset.with_ymd_and_hms(2026, 1, 24, 23, 25, 32).unwrap().with_nanosecond(132936000).unwrap(), 35.76f32),
                (offset.with_ymd_and_hms(2026, 1, 25, 1, 22, 32).unwrap().with_nanosecond(132936000).unwrap(), 36.67f32),
                (offset.with_ymd_and_hms(2026, 1, 25, 3, 13, 32).unwrap().with_nanosecond(132936000).unwrap(), 36.87f32),
                (offset.with_ymd_and_hms(2026, 1, 25, 4, 49, 32).unwrap().with_nanosecond(132936000).unwrap(), 37.56f32),
                (offset.with_ymd_and_hms(2026, 1, 25, 5, 10, 32).unwrap().with_nanosecond(132936000).unwrap(), 37.69f32),
                (offset.with_ymd_and_hms(2026, 1, 25, 6, 45, 32).unwrap().with_nanosecond(132936000).unwrap(), 38.36f32),
                (offset.with_ymd_and_hms(2026, 1, 25, 7, 59, 32).unwrap().with_nanosecond(132936000).unwrap(), 38.96f32),
                (offset.with_ymd_and_hms(2026, 1, 25, 9, 48, 32).unwrap().with_nanosecond(132936000).unwrap(), 39.66f32),
            ]
        );
    }
}
