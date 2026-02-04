//! 服务端逻辑.
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Cursor, SeekFrom};
use std::ops::{DerefMut, Sub};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::extract::DefaultBodyLimit;
use axum::http::header::COOKIE;
use axum::{
    Router,
    routing::{get, post},
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
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};

use crate::config::{
    RECORDS_FILENAME, ROOM_CONFIG_FILENAME, ROOM_UNKNOWN_DIRNAME, RoomConfig,
    SERVER_CONFIG_FILENAME, ServerConfig, config_dir, data_dir, log_dir,
};
use crate::error::{CSError, Error};
use crate::rooms::{Buildings, Districts, Floors, RoomInfo, Rooms};
use crate::{Cookies, Records};

mod log;
pub(crate) mod route;

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
    room_config: RoomConfig,
    cookies: Cookies,
    client: Client,
    room_info_cache: Mutex<HashMap<RoomConfig, RoomInfo>>,
}

impl Debug for Querier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Querier")
            .field("room_config", &self.room_config)
            .field("cookies", &self.cookies)
            .field("client", &self.client)
            .finish()
    }
}

impl Querier {
    #[must_use]
    fn new(config: RoomConfig) -> Querier {
        Querier {
            room_config: config,
            ..Default::default()
        }
    }

    #[inline]
    fn set_room_config(&mut self, config: RoomConfig) {
        self.room_config = config;
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
    async fn query_electricity_degree(&self) -> crate::Result<f32> {
        let payload = json!({
            "sysid": 1,
            "roomNo": self.room_config.room_no.as_str(),
            "elcarea": self.room_config.elcarea,
            "elcbuis": self.room_config.elcbuis.as_str(),
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

    pub const QUERY_DISTRICT_URL: &str =
        "https://epay.ecnu.edu.cn/epaycas/electric/queryelectricarea";
    pub const QUERY_BUILDINGS_URL: &str =
        "https://epay.ecnu.edu.cn/epaycas/electric/queryelectricbuis";
    pub const QUERY_FLOORS_URL: &str =
        "https://epay.ecnu.edu.cn/epaycas/electric/queryelectricfloors";
    pub const QUERY_ROOMS_URL: &str =
        "https://epay.ecnu.edu.cn/epaycas/electric/queryelectricrooms";

    /// 获取学校宿舍的信息 (宿舍代码及其对应的可视值).
    pub async fn get_room_info(&self) -> crate::Result<RoomInfo> {
        if !self.room_config.is_invalid()
            && let Some(room_info) = self.room_info_cache.lock().await.get(&self.room_config)
        {
            return Ok(room_info.clone());
        }
        let parts: [&str; 4] = *self
            .room_config
            .room_no
            .splitn(4, '_')
            .collect::<Vec<&str>>()
            .as_array::<4>()
            .ok_or(CSError::InvalidRoomConfig)?;
        // dd_XX_dd_dd
        let room_name = parts[0];
        let district_id = parts[1];
        let floor_id = parts[3];
        let building_id: &str = &self.room_config.elcbuis;
        let area_id: &str = &self.room_config.elcarea.to_string();

        // 在这里是直接请求学校的网站, 因此不需要 self.client, 也不能使用(TLS 配置不兼容).
        // 但是需要附上 cookies 数据.
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            COOKIE,
            format!(
                "JSESSIONID={}; cookie={}",
                self.cookies.j_session_id, self.cookies.cookie
            )
            .parse()
            .map_err(|_| CSError::InvalidCookies)?,
        );
        headers.insert(
            "X-CSRF-TOKEN",
            self.cookies
                .x_csrf_token
                .parse()
                .map_err(|_| CSError::InvalidCookies)?,
        );
        let mut districts: Districts = self
            .client
            .post(Self::QUERY_DISTRICT_URL)
            .headers(headers.clone())
            .form(&[
                // sysid=1 表示查询电费系统, sysid=2 表示查询水费系统, 后者这里不需要.
                ("sysid", "1"),
            ])
            .send()
            .await?
            .json()
            .await
            .map_err(|e| {
                error!("invalid district response, maybe permission denied: {e:?}");
                CSError::EcnuNotLogin
            })?;
        let district = districts
            .districts
            .into_iter()
            .find(|d| d.district_id == district_id)
            .ok_or(CSError::RoomInfoNotFound)?;

        let buildings: Buildings = self
            .client
            .post(Self::QUERY_BUILDINGS_URL)
            .headers(headers.clone())
            .form(&[("sysid", "1"), ("area", area_id), ("district", district_id)])
            .send()
            .await?
            .json()
            .await
            .map_err(|e| {
                error!("invalid building response, maybe permission denied: {e:?}");
                CSError::EcnuNotLogin
            })?;
        let building = buildings
            .buildings
            .into_iter()
            .find(|b| b.building_id == building_id)
            .ok_or(CSError::RoomInfoNotFound)?;

        let floors: Floors = self
            .client
            .post(Self::QUERY_FLOORS_URL)
            .headers(headers.clone())
            .form(&[
                ("sysid", "1"),
                ("area", area_id),
                ("district", district_id),
                ("build", building_id),
            ])
            .send()
            .await?
            .json()
            .await
            .map_err(|e| {
                error!("invalid floors response, maybe permission denied: {e:?}");
                CSError::EcnuNotLogin
            })?;
        let floor = floors
            .floors
            .into_iter()
            .find(|f| f.floor_id == floor_id)
            .ok_or(CSError::RoomInfoNotFound)?;

        let rooms: Rooms = self
            .client
            .post(Self::QUERY_ROOMS_URL)
            .headers(headers.clone())
            .form(&[
                ("sysid", "1"),
                ("area", area_id),
                ("district", district_id),
                ("build", building_id),
                ("floor", floor_id),
            ])
            .send()
            .await?
            .json()
            .await
            .map_err(|e| {
                error!("invalid room response, maybe permission denied: {e:?}");
                CSError::EcnuNotLogin
            })?;
        let room = rooms
            .rooms
            .into_iter()
            .find(|f| f.room_name == room_name)
            .ok_or(CSError::RoomInfoNotFound)?;

        let room_info = RoomInfo {
            area: districts.areas.pop().ok_or(CSError::RoomInfoNotFound)?,
            district,
            building,
            floor,
            room,
        };
        let mut room_info_cache = self.room_info_cache.lock().await;
        room_info_cache.insert(self.room_config.clone(), room_info);
        Ok(room_info_cache.get(&self.room_config).unwrap().clone())
    }
}

struct Recorder {
    out: RwLock<File>,
    /// 最后一个记录的电量, 保证已经被输出到 out 之中.
    last_degree: Option<f32>,
}

impl Debug for Recorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recorder")
            .field("out", &"...")
            .field("last_degree", &self.last_degree)
            .finish()
    }
}

/// 在 Recorder<File> 运行时修改对应的 csv 文件可能会破坏结果/无法及时得到响应.
impl Recorder {
    /// 写入一个时间, 剩余度数对到输出中.
    async fn record_instant(
        &mut self,
        time: DateTime<FixedOffset>,
        degree: f32,
    ) -> crate::Result<()> {
        let line = format!("{},{}\n", time.to_rfc3339(), degree);
        self.out.write().await.write_all(line.as_bytes()).await?;
        self.last_degree = Some(degree);
        Ok(())
    }

    async fn record_multiple(&mut self, records: Records) -> crate::Result<()> {
        let mut out = self.out.write().await;
        let last_degree = records.last().map(|x| x.1).or(self.last_degree);
        for (time, degree) in records.0 {
            let line = format!("{},{}\n", time.to_rfc3339(), degree);
            out.write_all(line.as_bytes()).await?;
        }
        self.last_degree = last_degree;
        Ok(())
    }

    /// 尝试记录一次电量变化, 只有产生了电量度数的变化才会被记录, 如果被记录了, 那么返回 Ok(true).
    async fn record(&mut self, degree: f32) -> crate::Result<bool> {
        let now_time = Local::now().fixed_offset().with_nanosecond(0).unwrap();
        if let Some(last_degree) = self.last_degree
            && last_degree.sub(degree).abs() < 0.01
        {
            return Ok(false);
        }

        self.record_instant(now_time, degree).await?;
        Ok(true)
    }

    /// 从可读可写文件中加载.
    async fn load_from_rw_file(mut file: File) -> crate::Result<Recorder> {
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
            out: RwLock::new(file),
            last_degree,
        })
    }

    /// 从路径中加载, 如果文件不存在, 文件将被创建, 并返回对应没有任何记录 Recorder.
    async fn load_from_path(records_path: impl AsRef<Path>) -> crate::Result<Recorder> {
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

    /// 将符合时间范围的记录摘取出来, 从 records.csv 中去除, 此函数不会立即操作, 而是先预览 archived 之后的分割结果.
    ///
    /// # Returns
    ///
    /// [`ArchiveHandle`],
    ///
    /// # Errors
    ///
    /// - 需要 File 输出对象是 Seekable 和 Readable 的, 不然将会返回 [`Error::Io`].
    async fn archive(&mut self, time_span: TimeSpan) -> crate::Result<ArchiveHandle<'_>> {
        let mut out = self.out.write().await;
        out.seek(SeekFrom::Start(0)).await?;
        let records = Records::from_csv(&mut out.deref_mut()).await?;
        out.seek(SeekFrom::End(0)).await?;
        drop(out);
        let mut archived = Vec::new();
        let mut retained = Vec::new();
        for rec in records.0 {
            if time_span.contains(&rec.0) {
                archived.push(rec);
            } else {
                retained.push(rec);
            }
        }

        Ok(ArchiveHandle {
            recorder: self,
            retained: Records(retained),
            archived: Records(archived),
        })
    }

    /// 从文件中读取已经输出的 records.
    async fn read_records(&self) -> crate::Result<Records> {
        let mut out = self.out.write().await;
        out.seek(SeekFrom::Start(0)).await?;
        let rst = Records::from_csv(&mut out.deref_mut()).await;
        out.seek(SeekFrom::End(0)).await?;
        rst
    }
}

struct ArchiveHandle<'a> {
    recorder: &'a mut Recorder,
    /// 保持在 records.csv 中的记录
    retained: Records,
    /// 被归档的记录
    archived: Records,
}

impl<'a> ArchiveHandle<'a> {
    /// 确认 archive 操作, 如果不执行此方法, [`Recorder::archive`] 是无任何效果的.
    async fn commit(self) -> crate::Result<()> {
        let mut out = self.recorder.out.write().await;
        out.set_len(0).await?;
        out.seek(SeekFrom::Start(0)).await?;
        drop(out);
        self.recorder.last_degree = None;
        self.recorder.record_multiple(self.retained).await?;
        Ok(())
    }
}

#[derive(Debug)]
struct AppState {
    querier: RwLock<Querier>,
    recorder: RwLock<Recorder>,
    config_dir: PathBuf,
    // 当前宿舍房间的数据保存路径.
    room_dir: RwLock<PathBuf>,
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
    let data_dir = data_dir().with_context(|| "failed to access data dir")?;
    let config_dir = config_dir().with_context(|| "failed to access config dir")?;
    let log_dir = log_dir().with_context(|| "failed to access log dir")?;

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

    let querier = if let Ok(room_config) = room_config.as_ref() {
        Querier::new(room_config.clone())
    } else {
        Querier::default()
    };
    let room_dir = room_config
        .as_ref()
        .map(|rc| rc.dir())
        // 可以不存在房间配置.
        .unwrap_or_else(|_| Ok(data_dir.join(ROOM_UNKNOWN_DIRNAME)))?; // 但是不能是无效的房间配置
    fs::create_dir_all(&room_dir)
        .await
        .with_context(|| "failed to create room dir")?;
    let recorder = Recorder::load_from_path(room_dir.join(RECORDS_FILENAME))
        .await
        .with_context(|| "failed to initialize recorder")?;
    let app_state = Arc::new(AppState {
        querier: RwLock::new(querier),
        recorder: RwLock::new(recorder),
        config_dir,
        room_dir: RwLock::new(room_dir),
    });
    use route::*;
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
        .route("/get-room", get(get_room))
        .route("/get-room-info", get(get_room_info))
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
            archived.archived.0,
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
