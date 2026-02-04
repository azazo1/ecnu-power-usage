use std::{io, path::PathBuf, string::FromUtf8Error};

use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("ecnu error: {0}")]
    Ecnu(String),
    #[error("response has no degree provided.")]
    NoDegree,
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error("file {0} read error: {1}")]
    FileRead(PathBuf, String),
    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),
    #[error(transparent)]
    FloatParse(#[from] std::num::ParseFloatError),
    #[error("invalid degree records format")]
    InvalidRecordsFormat,
    #[error(transparent)]
    Chromium(#[from] chromiumoxide::error::CdpError),
    #[error("browser page error: {0}")]
    BrowserPage(String),
    #[error("browser cookie error: {0}")]
    Cookie(String),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
    #[error(transparent)]
    Csv(#[from] csv_async::Error),
    #[error("cs response: {0}")]
    CS(#[from] CSError),
    #[error(transparent)]
    Utf8(#[from] FromUtf8Error),
    #[error(transparent)]
    Log(#[from] tracing_appender::rolling::InitError),
}

/// Client-Server error
#[derive(thiserror::Error, Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "tag", content = "content")]
pub enum CSError {
    #[error("{0}")]
    General(String),
    #[error("ecnu is not logged in on server side")]
    EcnuNotLogin,
    #[error("server lacks room config")]
    RoomConfigMissing,
    #[error("archive is empty")]
    EmptyArchive,
    #[error("archive dir is not exist or cannot be read")]
    ArchiveDir,
    #[error("room dir is not exist or cannot be read")]
    RoomDir,
    #[error("listing archives failed")]
    ListArchive,
    #[error("saving archive meta failed")]
    SaveArchiveMeta,
    #[error("serializing archive meta failed")]
    SerializeMeta,
    #[error("writing archive file failed")]
    WriteArchive,
    #[error("serializing records failed")]
    SerializeRecords,
    #[error("duplicated archive name")]
    DuplicatedArchive,
    #[error("reading records failed")]
    ReadRecords,
    #[error("logged into ecnu, but failed to query degree")]
    QueryDegree,
    #[error("saving room config failed")]
    SaveRoomConfig,
    #[error("invalid archive name")]
    InvalidArchiveName,
    #[error("archive not found")]
    ArchiveNotFound,
    #[error("failed to delete archive")]
    DeletedArchiveFailed,
    #[error("invalid room config")]
    InvalidRoomConfig,
    #[error("room info not found in the ecnu database")]
    RoomInfoNotFound,
    #[error("invalid cookies provided")]
    InvalidCookies,
    #[error("server failed to send request")]
    ServerRequestError,
}

pub type Result<T> = std::result::Result<T, Error>;
pub type CSResult<T> = std::result::Result<T, CSError>;
