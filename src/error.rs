use std::{io, path::PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("ecnu error: {0}")]
    EcnuError(String),
    #[error("response has no degree provided.")]
    NoDegree,
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error("config {0} read error: {1}")]
    ConfigFileReadError(PathBuf, String),
    #[error(transparent)]
    ChronoParseError(#[from] chrono::ParseError),
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error("invalid degree records format")]
    DegreeRecordsFormatError,
    #[error(transparent)]
    ChromiumError(#[from] chromiumoxide::error::CdpError),
    #[error("{0}")]
    ChromiumParamBuildingError(String),
    #[error("browser page error: {0}")]
    BrowserPageError(String),
    #[error("browser cookie error: {0}")]
    CookieError(String),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    CsvError(#[from] csv_async::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
