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
    #[error("listing archives failed")]
    ListArchive,
    #[error("saving archive meta failed")]
    SaveMeta,
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
}

pub type Result<T> = std::result::Result<T, Error>;
pub type CSResult<T> = std::result::Result<T, CSError>;
