use std::{io, path::PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("ecnu error: {0}")]
    EcnuError(String),
    #[error("response has no degree provided.")]
    NoDegree,
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error("config {0} read error: {1}")]
    ConfigFileReadError(PathBuf, String),
}

pub type Result<T> = std::result::Result<T, Error>;
