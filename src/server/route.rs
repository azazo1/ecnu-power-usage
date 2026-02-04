use axum::{Form, Json, response::IntoResponse};
use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::fs::{self, File};
use tokio_util::io::ReaderStream;
use tracing::{debug, error, info, warn};

use crate::config::{
    ARCHIVE_DIRNAME, DELETED_DIRNAME, RECORDS_FILENAME, ROOM_CONFIG_FILENAME, RoomConfig,
    is_sanitized_filename,
};
use crate::error::{CSError, CSResult, Error};
use crate::rooms::RoomInfo;
use crate::{ArchiveMeta, Cookies, Records, TimeSpan};

use crate::server::{AppState, Recorder};

pub(super) async fn post_room(
    State(state): State<Arc<AppState>>,
    Json(room_config): Json<RoomConfig>,
) -> (StatusCode, Json<CSResult<()>>) {
    info!("post room request.");
    if !is_sanitized_filename(&room_config.room_no) {
        return (
            StatusCode::BAD_REQUEST,
            Json(Err(CSError::InvalidRoomConfig)),
        );
    }
    let room_dir = match room_config.dir() {
        Ok(x) => x,
        Err(e) => {
            error!(target: "creating room dir", "{e:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::RoomDir)),
            );
        }
    };
    let recorder = match Recorder::load_from_path(room_dir.join(RECORDS_FILENAME)).await {
        Ok(x) => x,
        Err(Error::CS(e)) => return (StatusCode::BAD_REQUEST, Json(Err(e))),
        Err(e) => {
            error!(target: "loading recorder", "{e:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::ReadRecords)),
            );
        }
    };
    if let Err(e) = room_config
        .save_to_file(state.config_dir.join(ROOM_CONFIG_FILENAME))
        .await
    {
        error!(target: "saving room config",  "{e:?}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Err(CSError::SaveRoomConfig)),
        );
    }
    // 原子化操作, 推迟到这里赋值.
    *state.room_dir.write().await = room_dir;
    *state.recorder.write().await = recorder;
    state
        .querier
        .write()
        .await
        .set_room_config(room_config.clone());
    (StatusCode::OK, Json(Ok(())))
}

pub(super) async fn post_cookies(
    State(state): State<Arc<AppState>>,
    Json(cookies): Json<Cookies>,
) -> StatusCode {
    info!("post cookies request.");
    state.querier.write().await.refresh(cookies);
    StatusCode::OK
}

pub(super) async fn get_records(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<CSResult<Records>>) {
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

pub(super) async fn get_degree(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<CSResult<f32>>) {
    debug!("get degree request.");
    let querier = state.querier.read().await;
    if querier.room_config.is_invalid() {
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
pub(crate) struct CreateArchiveArgs {
    pub(crate) time_span: TimeSpan,
    /// 默认名称为创建的 archive 时间跨度.
    pub(crate) archive_name: Option<String>,
}

/// 创建 archive, 将符合时间范围的 records 保存到 archives 之中.
///
/// fixme: 这里的原子化实践仍然可能由于 async cancellation 而取消, 这点需要解决.
pub(super) async fn create_archive(
    State(state): State<Arc<AppState>>,
    Json(args): Json<CreateArchiveArgs>,
) -> (StatusCode, Json<CSResult<ArchiveMeta>>) {
    info!("create archive request: {args:#?}");

    let CreateArchiveArgs {
        time_span,
        archive_name,
    } = args;

    if let Some(archive_name) = archive_name.as_ref()
        && !is_sanitized_filename(archive_name)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(Err(CSError::InvalidArchiveName)),
        );
    }

    let mut recorder = state.recorder.write().await;
    let mut handle = match recorder.archive(time_span).await {
        Ok(x) => x,
        Err(e) => {
            error!(target: "reading records", "{e:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::ReadRecords)),
            );
        }
    };

    handle.archived.sort();
    let (start_time, end_time) = match handle.archived.time_span() {
        Some(x) => x,
        None => {
            // 如果 records 无法计算出时间跨度, 那么说明其为空.
            return (StatusCode::OK, Json(Err(CSError::EmptyArchive)));
        }
    };

    let archive_name = match archive_name {
        Some(x) => x,
        None => {
            format!(
                "{}-{}-by-{}",
                start_time.format("%Y%d%m"),
                end_time.format("%Y%d%m"),
                Local::now().format("%Y%d%m_%H%M%S")
            )
        }
    };

    let archive_dir = state.room_dir.read().await.join(ARCHIVE_DIRNAME);
    let archive_file = archive_dir.join(format!("{}.csv", archive_name));
    let archive_meta_file = archive_dir.join(format!("{}.toml", archive_name));

    fs::create_dir_all(&archive_dir).await.ok(); // 如果失败了会在下面报错;

    match fs::read_dir(&archive_dir).await {
        Ok(mut rd) => {
            while let Some(entry) = match rd.next_entry().await {
                Ok(x) => x,
                Err(e) => {
                    error!(target: "reading archive dir", "{e:?}");
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
        records_num: handle.archived.len(),
    };

    let archived_content = match handle.archived.to_csv().await {
        Ok(x) => x,
        Err(e) => {
            error!(target: "serializing records", "{e:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::SerializeRecords)),
            );
        }
    };

    let archived_meta_content = match toml::to_string_pretty(&archive_meta) {
        Ok(x) => x,
        Err(e) => {
            error!(target: "serializing archive meta", "{e:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::SerializeMeta)),
            );
        }
    };

    // fixme: 这下面的操作仍然有可能被 async cancelled 而导致非原子化, 但是暂时无法使用 tokio::spawn 解决, 因为
    // state 和 recorder 的生命周期不够长 (handle 需要).
    if let Err(e) = fs::write(&archive_file, archived_content).await {
        error!(target: "writing archive file", "{e:?}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Err(CSError::WriteArchive)),
        );
    }

    if let Err(e) = fs::write(&archive_meta_file, archived_meta_content).await {
        error!(target: "saving archive meta", "{e:?}");
        fs::remove_file(&archive_file).await.ok();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Err(CSError::SaveArchiveMeta)),
        );
    }

    if let Err(e) = handle.commit().await {
        error!(target: "commiting archive", "{e:?}");
        fs::remove_file(archive_file).await.ok();
        fs::remove_file(archive_meta_file).await.ok();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Err(CSError::WriteArchive)),
        );
    }
    (StatusCode::OK, Json(Ok(archive_meta)))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct DownloadArchiveArgs {
    pub(crate) name: String,
}

/// 这里的 Form 需要使用 reqwest .query() 的方式给入, 而不是 .form().
pub(super) async fn download_archive(
    State(state): State<Arc<AppState>>,
    Form(args): Form<DownloadArchiveArgs>,
) -> Response<Body> {
    info!("download archive request: {}", args.name);

    let archive_dir = state.room_dir.read().await.join(ARCHIVE_DIRNAME);

    let archive_name = if is_sanitized_filename(&args.name) {
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

pub(super) async fn list_archives(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<CSResult<Vec<ArchiveMeta>>>) {
    info!("list archives request.");

    let archive_dir = state.room_dir.read().await.join(ARCHIVE_DIRNAME);
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

pub(super) async fn delete_archive(
    State(state): State<Arc<AppState>>,
    Form(args): Form<DeleteArchiveArgs>,
) -> (StatusCode, Json<CSResult<()>>) {
    info!("delete archive request: {args:#?}");
    let DeleteArchiveArgs { name: archive_name } = args;
    if !is_sanitized_filename(&archive_name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(Err(CSError::InvalidArchiveName)),
        );
    }
    let room_dir = state.room_dir.read().await.clone();
    let archive_dir = room_dir.join(ARCHIVE_DIRNAME);
    let deleted_dir = room_dir.join(DELETED_DIRNAME);
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

pub(super) async fn clear_cookies(State(state): State<Arc<AppState>>) -> StatusCode {
    info!("clear cookies request.");
    state.querier.write().await.refresh(Cookies::empty());
    StatusCode::OK
}

pub(super) async fn clear_room(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<CSResult<()>>) {
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

pub(super) async fn get_room(State(state): State<Arc<AppState>>) -> (StatusCode, Json<RoomConfig>) {
    (
        StatusCode::OK,
        Json(state.querier.read().await.room_config.clone()),
    )
}

pub(super) async fn get_room_info(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<CSResult<RoomInfo>>) {
    info!("get room info request.");
    match state.querier.read().await.get_room_info().await {
        Ok(room_info) => (StatusCode::OK, Json(Ok(room_info))),
        Err(Error::CS(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(Err(e))),
        Err(e) => {
            warn!("getting room info: {e:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Err(CSError::ServerRequestError)),
            )
        }
    }
}
