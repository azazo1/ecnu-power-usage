use tokio::io;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error("initializing rolling file appender: {0}")]
    Log(#[from] tracing_appender::rolling::InitError),
    #[error(transparent)]
    Lib(#[from] ecnu_power_usage::Error),
    #[error("certificate file is too large")]
    CertFileTooLarge,
    #[error("certificate path is not file")]
    CertNotFile,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error("display detecting: {0}")]
    Display(i32),
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
