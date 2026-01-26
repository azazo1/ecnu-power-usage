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
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
