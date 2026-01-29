//! 服务端逻辑.
use std::fmt::Debug;
use std::io::{Cursor, SeekFrom};
use std::ops::Sub;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::extract::DefaultBodyLimit;
use axum::{
    Form, Json, Router,
    response::IntoResponse,
    routing::{get, post},
};
use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode, header::COOKIE},
};
use axum_server::tls_rustls::RustlsConfig;
use chrono::{DateTime, FixedOffset, Local, TimeZone, Timelike};
use reqwest::{Client, Method};
use rustls::RootCertStore;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs::{self, File};
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use tracing::{debug, error, info, warn};

use crate::config::{
    ARCHIVE_DIRNAME, DELETED_DIRNAME, LOG_DIRNAME, RECORDS_FILENAME, ROOM_CONFIG_FILENAME,
    RoomConfig, SERVER_CONFIG_FILENAME, ServerConfig,
};
use crate::error::{CSError, CSResult, Error, Result};
use crate::{Cookies, Records};

mod log;

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
struct Querier {
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
    fn new(config: RoomConfig) -> Querier {
        Querier {
            config,
            cookies: Default::default(),
            client: Default::default(),
        }
    }

    #[inline]
    fn set_room_config(&mut self, config: RoomConfig) {
        self.config = config;
    }

    /// 重新设置有效的 cookies.
    fn refresh(&mut self, cookies: Cookies) {
        self.cookies = cookies.sanitize();
    }

    /// 查询查询当前剩余电量 (度)
    /// # Errors
    /// - [`Error::Reqwest`][]: see: [`reqwest::RequestBuilder::send`].
    /// - [`Error::Ecnu`][]: ECNU 未登录 / 查询接口返回错误信息.
    /// - [`Error::NoDegree`][]: 不应出现此情况, 如果出现了可能是接口返回了错误的数据.
    async fn query_electricity_degree(&self) -> Result<f32> {
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
            Err(Error::Ecnu("permission denied".to_string()))?
        }
        let ret: QueryResponse = resp.json().await?;
        if ret.code != 0 || ret.msg != "成功" {
            Err(Error::Ecnu(ret.msg))?
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

    /// 尝试记录一次电量变化, 只有产生了电量度数的变化才会被记录, 如果被记录了, 那么返回 Ok(true).
    async fn record(&mut self, degree: f32) -> Result<bool> {
        let now_time = Local::now().fixed_offset().with_nanosecond(0).unwrap();
        if let Some(last_degree) = self.last_degree
            && last_degree.sub(degree).abs() < 0.01
        {
            return Ok(false);
        }

        self.record_instant(now_time, degree).await?;
        Ok(true)
    }
}

/// 在 Recorder<File> 运行时修改对应的 csv 文件可能会破坏结果/无法及时得到响应.
impl Recorder<File> {
    /// 从可读可写文件中加载.
    async fn load_from_rw_file(mut file: File) -> Result<Recorder<File>> {
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
                .ok_or(Error::InvalidRecordsFormat)?;
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
    async fn load_from_path(records_path: impl AsRef<Path>) -> Result<Recorder<File>> {
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
    /// - 需要 File 输出对象是 Seekable 和 Readable 的, 不然将会返回 [`Error::Io`].
    async fn archive(&mut self, time_span: TimeSpan) -> Result<Records> {
        self.out.seek(SeekFrom::Start(0)).await?;
        let records = Records::from_csv(&mut self.out).await?;
        let mut archived = Vec::new();
        let mut retained = Vec::new();
        for rec in records.0 {
            if time_span.contains(&rec.0) {
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
    async fn read_records(&mut self) -> Result<Records> {
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

fn is_valid_archive_name(name: &str) -> bool {
    let archive_name = Path::new(name).file_name().and_then(|s| s.to_str());
    sanitize_filename::is_sanitized(name)
        && archive_name.is_some_and(|f| f == name)
        && name.find(['/', '\\']).is_none()
}

async fn post_room(
    State(state): State<Arc<AppState>>,
    Json(room_config): Json<RoomConfig>,
) -> (StatusCode, Json<CSResult<()>>) {
    info!("post room request.");
    state
        .querier
        .write()
        .await
        .set_room_config(room_config.clone());
    if let Err(e) = room_config
        .save_to_file(state.config_dir.join(ROOM_CONFIG_FILENAME))
        .await
    {
        error!("saving room config: {e:?}");
        (StatusCode::OK, Json(Err(CSError::SaveRoomConfig)))
    } else {
        (StatusCode::OK, Json(Ok(())))
    }
}

async fn post_cookies(
    State(state): State<Arc<AppState>>,
    Json(cookies): Json<Cookies>,
) -> StatusCode {
    info!("post cookies request.");
    state.querier.write().await.refresh(cookies);
    StatusCode::OK
}

async fn get_records(State(state): State<Arc<AppState>>) -> (StatusCode, Json<CSResult<Records>>) {
    debug!("get records request.");
    match state.recorder.write().await.read_records().await {
        Ok(records) => (StatusCode::OK, Json(Ok(records))),
        Err(e) => {
            error!("reading records from file: {e:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::ReadRecords)),
            )
        }
    }
}

async fn get_degree(State(state): State<Arc<AppState>>) -> (StatusCode, Json<CSResult<f32>>) {
    debug!("get degree request.");
    let querier = state.querier.read().await;
    if querier.config.is_invalid() {
        return (StatusCode::OK, Json(Err(CSError::RoomConfigMissing)));
    }
    match querier.query_electricity_degree().await {
        Ok(degree) => (StatusCode::OK, Json(Ok(degree))),
        Err(e) => match e {
            Error::Ecnu(_) => (StatusCode::OK, Json(Err(CSError::EcnuNotLogin))),
            e => {
                error!("querying degree: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(Err(CSError::QueryDegree)),
                )
            }
        },
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TimeSpan {
    /// 时间范围: 开头 (包含)
    pub start_time: Option<DateTime<FixedOffset>>,
    /// 时间范围: 末尾 (包含)
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
        self.start_time.is_none_or(|st| st.le(o)) && self.end_time.is_none_or(|et| et.ge(o))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct CreateArchiveArgs {
    pub(crate) time_span: TimeSpan,
    /// 默认名称为创建的 archive 时间跨度.
    pub(crate) archive_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ArchiveMeta {
    // 下面这俩时间和 timespan 的区别是其不会为 None.
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub archive_name: String,
    pub records_num: usize,
}

impl PartialEq for ArchiveMeta {
    fn eq(&self, other: &Self) -> bool {
        self.start_time == other.start_time
            && self.end_time == other.end_time
            && self.archive_name == other.archive_name
            && self.records_num == other.records_num
    }
}

impl Eq for ArchiveMeta {}

impl Ord for ArchiveMeta {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.start_time.cmp(&other.start_time) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.archive_name.cmp(&other.archive_name) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.end_time.cmp(&other.end_time) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.records_num.cmp(&other.records_num)
    }
}

impl PartialOrd for ArchiveMeta {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// 创建 archive, 将符合时间范围的 records 保存到 archives 之中.
async fn create_archive(
    State(state): State<Arc<AppState>>,
    Json(args): Json<CreateArchiveArgs>,
) -> (StatusCode, Json<CSResult<ArchiveMeta>>) {
    info!("create archive request: {args:#?}");

    let CreateArchiveArgs {
        time_span,
        archive_name,
    } = args;

    let mut recorder = state.recorder.write().await;
    let mut archived = match recorder.archive(time_span).await {
        Ok(x) => x,
        Err(e) => {
            error!("reading records: {e:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::ReadRecords)),
            );
        }
    };

    archived.sort();
    let (start_time, end_time) = match archived.time_span() {
        Some(x) => x,
        None => {
            // 如果 records 无法计算出时间跨度, 那么说明其为空.
            return (StatusCode::OK, Json(Err(CSError::EmptyArchive)));
        }
    };

    let archive_name = match archive_name {
        Some(x) if is_valid_archive_name(&x) => x,
        None => {
            format!(
                "{}-{}-by-{}",
                start_time.format("%Y%d%m"),
                end_time.format("%Y%d%m"),
                Local::now().format("%Y%d%m_%H%M%S")
            )
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Err(CSError::InvalidArchiveName)),
            );
        }
    };

    let archive_dir = state.data_dir.join(ARCHIVE_DIRNAME);
    let archive_file = archive_dir.join(format!("{}.csv", archive_name));
    let archive_meta_file = archive_dir.join(format!("{}.toml", archive_name));

    fs::create_dir_all(&archive_dir).await.ok(); // 如果失败了会在下面报错;

    match fs::read_dir(&archive_dir).await {
        Ok(mut rd) => {
            while let Some(entry) = match rd.next_entry().await {
                Ok(x) => x,
                Err(e) => {
                    error!("reading archive dir: {e:?}");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(Err(CSError::ArchiveDir)),
                    );
                }
            } {
                if entry.file_name() == archive_file {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(Err(CSError::DuplicatedArchive)),
                    );
                }
            }
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::ArchiveDir)),
            );
        }
    }

    let archive_meta = ArchiveMeta {
        start_time,
        end_time,
        archive_name,
        records_num: archived.0.len(),
    };

    let archived_content = match archived.to_csv().await {
        Ok(x) => x,
        Err(e) => {
            error!("serializing records: {e:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::SerializeRecords)),
            );
        }
    };

    if let Err(e) = fs::write(archive_file, archived_content).await {
        error!("writing archive file: {e:?}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Err(CSError::WriteArchive)),
        );
    }

    let archived_meta_content = match toml::to_string_pretty(&archive_meta) {
        Ok(x) => x,
        Err(e) => {
            error!("serializing archive meta: {e:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::SerializeMeta)),
            );
        }
    };

    if let Err(e) = fs::write(archive_meta_file, archived_meta_content).await {
        error!("saving archive meta: {e:?}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Err(CSError::SaveMeta)),
        );
    }

    (StatusCode::OK, Json(Ok(archive_meta)))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct DownloadArchiveArgs {
    pub(crate) name: String,
}

/// 这里的 Form 需要使用 reqwest .query() 的方式给入, 而不是 .form().
async fn download_archive(
    State(state): State<Arc<AppState>>,
    Form(args): Form<DownloadArchiveArgs>,
) -> Response<Body> {
    info!("download archive request: {}", args.name);

    let archive_dir = state.data_dir.join(ARCHIVE_DIRNAME);

    let archive_name = if is_valid_archive_name(&args.name) {
        args.name
    } else {
        return (StatusCode::BAD_REQUEST, Json(CSError::InvalidArchiveName)).into_response();
    };

    match File::open(archive_dir.join(format!("{}.csv", archive_name))).await {
        Ok(file) => {
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/csv")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"{}\"", archive_name),
                )
                .body(body)
                .unwrap()
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, Json(CSError::ArchiveNotFound)).into_response(),
    }
}

async fn list_archives(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<CSResult<Vec<ArchiveMeta>>>) {
    info!("list archives request.");

    let archive_dir = state.data_dir.join(ARCHIVE_DIRNAME);
    fs::create_dir_all(&archive_dir).await.ok();
    let mut rd = match fs::read_dir(&archive_dir).await {
        Ok(x) => x,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::ArchiveDir)),
            );
        }
    };
    let mut archive_metas = Vec::new();
    while let Some(entry) = match rd.next_entry().await {
        Ok(x) => x,
        Err(e) => {
            error!("listing archives: {e:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::ListArchive)),
            );
        }
    } {
        let Some(meta_filename) = entry
            .file_name()
            .to_str()
            .filter(|s| s.ends_with(".toml"))
            .map(|s: &str| s.to_string())
        else {
            continue;
        };
        let meta_file = archive_dir.join(&meta_filename);
        let meta_content = match fs::read(meta_file).await {
            Ok(x) => x,
            Err(_) => continue,
        };
        let meta: ArchiveMeta = match toml::from_slice(&meta_content) {
            Ok(x) => x,
            Err(_) => continue,
        };
        archive_metas.push(meta);
    }

    archive_metas.sort();
    (StatusCode::OK, Json(Ok(archive_metas)))
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct DeleteArchiveArgs {
    pub(crate) name: String,
}

async fn delete_archive(
    State(state): State<Arc<AppState>>,
    Form(args): Form<DeleteArchiveArgs>,
) -> (StatusCode, Json<CSResult<()>>) {
    info!("delete archive request: {args:#?}");
    let DeleteArchiveArgs { name: archive_name } = args;
    if !is_valid_archive_name(&archive_name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(Err(CSError::InvalidArchiveName)),
        );
    }
    let archive_dir = state.data_dir.join(ARCHIVE_DIRNAME);
    let deleted_dir = state.data_dir.join(DELETED_DIRNAME);
    let archive_file = archive_dir.join(format!("{}.csv", archive_name));
    let archive_meta_file = archive_dir.join(format!("{}.toml", archive_name));

    if !archive_meta_file.exists() {
        return (StatusCode::NOT_FOUND, Json(Err(CSError::ArchiveNotFound)));
    }

    let now = Local::now();
    let now = now.format("%Y%m%d-%H%M");

    let mut deleted_archive_file = deleted_dir.join(format!("{}.csv.{}.{}", archive_name, now, 0));
    let mut deleted_archive_meta_file =
        deleted_dir.join(format!("{}.toml.{}.{}", archive_name, now, 0));
    // 已删除归档名称去重.
    while deleted_archive_file.exists() {
        let Some(prev_ext) = deleted_archive_file.extension().and_then(|s| s.to_str()) else {
            return (
                StatusCode::BAD_REQUEST,
                Json(Err(CSError::InvalidArchiveName)),
            );
        };
        let num = if let Ok(num) = prev_ext.parse::<usize>() {
            num + 1
        } else {
            1
        };
        deleted_archive_file = deleted_archive_file.with_extension(format!("{num}"));
        deleted_archive_meta_file = deleted_archive_meta_file.with_extension(format!("{num}"));
    }

    fs::create_dir_all(&archive_dir).await.ok();
    if let Err(e) = fs::create_dir_all(&deleted_dir).await {
        error!("create DELETED dir failed: {e:?}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Err(CSError::DeletedArchiveFailed)),
        );
    }
    info!("renaming: {archive_file:?} -> {deleted_archive_file:?}");
    if let Err(e) = fs::rename(&archive_file, &deleted_archive_file).await {
        error!("renaming failed: {e:?}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Err(CSError::DeletedArchiveFailed)),
        );
    }
    info!("renaming: {archive_meta_file:?} -> {deleted_archive_meta_file:?}");
    if let Err(e) = fs::rename(&archive_meta_file, &deleted_archive_meta_file).await {
        error!("renaming failed: {e:?}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Err(CSError::DeletedArchiveFailed)),
        );
    }
    (StatusCode::OK, Json(Ok(())))
}

async fn clear_cookies(State(state): State<Arc<AppState>>) -> StatusCode {
    info!("clear cookies request.");
    state.querier.write().await.refresh(Cookies::empty());
    StatusCode::OK
}

async fn clear_room(State(state): State<Arc<AppState>>) -> (StatusCode, Json<CSResult<()>>) {
    info!("clear room request.");
    state
        .querier
        .write()
        .await
        .set_room_config(RoomConfig::empty());
    if let Err(e) = RoomConfig::empty()
        .save_to_file(state.config_dir.join(ROOM_CONFIG_FILENAME))
        .await
    {
        error!("saving room config: {e:?}");
        (StatusCode::OK, Json(Err(CSError::SaveRoomConfig)))
    } else {
        (StatusCode::OK, Json(Ok(())))
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
                    if matches!(e, Error::Ecnu(_)) {
                        loop_state = LoopState::NotLogined;
                    }
                }
                LoopState::NotLogined => {
                    if !matches!(e, Error::Ecnu(_)) {
                        error!("querying: {e:?}");
                    }
                }
            },
        }
    }
}

/// 创建并启动服务.
pub async fn run_app() -> anyhow::Result<()> {
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    let default_data_dir = shellexpand::tilde("~/.local/share");
    let default_config_dir = shellexpand::tilde("~/.config");
    let data_dir = dirs_next::data_dir()
        .unwrap_or(default_data_dir.to_string().into())
        .join(PKG_NAME);
    let config_dir = dirs_next::config_dir()
        .unwrap_or(default_config_dir.to_string().into())
        .join(PKG_NAME);
    let log_dir = data_dir.join(LOG_DIRNAME);

    fs::create_dir_all(&data_dir)
        .await
        .with_context(|| data_dir.to_string_lossy().to_string())?;
    fs::create_dir_all(&config_dir)
        .await
        .with_context(|| config_dir.to_string_lossy().to_string())?;

    let _guard = log::init(&log_dir)
        .await
        .with_context(|| "init log failed")?;

    info!("data dir: {data_dir:?}");
    info!("config dir: {config_dir:?}");
    info!("log dir: {log_dir:?}");

    let room_config = RoomConfig::from_toml_file(config_dir.join(ROOM_CONFIG_FILENAME)).await;
    info!("room config: {room_config:#?}");

    let server_config_file = config_dir.join(SERVER_CONFIG_FILENAME);
    let server_config = ServerConfig::from_toml_file(&server_config_file, true).await?;
    info!("server config: {server_config:#?}");

    let app_state = Arc::new(AppState {
        querier: RwLock::new(if let Ok(room_config) = room_config {
            Querier::new(room_config)
        } else {
            Querier::default()
        }),
        recorder: RwLock::new(
            Recorder::load_from_path(data_dir.join(RECORDS_FILENAME))
                .await
                .with_context(|| {
                    data_dir
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
        .route("/create-archive", post(create_archive))
        .route("/get-records", get(get_records))
        .route("/get-degree", get(get_degree))
        .route("/download-archive", get(download_archive))
        .route("/list-archives", get(list_archives))
        .route("/delete-archive", post(delete_archive))
        .route("/clear-cookies", post(clear_cookies))
        .route("/clear-room", post(clear_room))
        .with_state(Arc::clone(&app_state))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024));
    let handle = tokio::spawn(async move { record_loop(app_state).await });

    if let Some(server_tls_config) = server_config.tls_config {
        // 加载 tls 服务

        let mut root_cert_store = RootCertStore::empty();
        for cert in load_certificate_der(&[&server_tls_config.root_ca]).await? {
            root_cert_store
                .add(cert)
                .with_context(|| "load root certs failed")?;
        }

        let cert_chain =
            load_certificate_der(&[&server_tls_config.server_cert, &server_tls_config.root_ca])
                .await?;

        let server_key = PrivateKeyDer::from_pem_reader(Cursor::new(
            fs::read(server_tls_config.server_key)
                .await
                .with_context(|| "read tls server key failed")?,
        ))?;

        // WebPkiClientVerifier 会强制要求客户端发送证书
        let client_verifier = WebPkiClientVerifier::builder(Arc::new(root_cert_store)).build()?;

        // 将客户端验证器注入到配置中
        let server_tls_config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(cert_chain, server_key)
            .with_context(|| "tls server config create failed")?;

        // 重新包装回 RustlsConfig
        let mtls_config = RustlsConfig::from_config(Arc::new(server_tls_config));

        axum_server::bind_rustls(server_config.bind_address, mtls_config)
            .serve(router.into_make_service())
            .await?;
    } else {
        warn!("launching without tls config, this service is only for dev usage.");
        let listener = TcpListener::bind(server_config.bind_address).await?;
        axum::serve(listener, router).await?;
    }

    handle.await?;
    Ok(())
}

async fn load_certificate_der(
    cert_paths: &[impl AsRef<Path>],
) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    let mut certs = Vec::new();
    for cert_path in cert_paths {
        let root_ca = fs::read(cert_path)
            .await
            .with_context(|| format!("read cert failed: {:?}", cert_path.as_ref()))?;
        let mut reader = Cursor::new(root_ca);
        for cert in rustls_pemfile::certs(&mut reader) {
            certs.push(cert.with_context(|| "invalid certificate")?);
        }
    }
    Ok(certs)
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
        for &(time, degree) in records.iter() {
            recorder.record_instant(time, degree).await.unwrap();
        }
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
