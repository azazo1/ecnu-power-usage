use crate::{CSError, error::Error};
use serde::{Deserialize, Serialize};
use std::{
    io,
    net::SocketAddr,
    path::{Path, PathBuf},
};
use tokio::fs;

pub(crate) fn is_sanitized_filename(name: &str) -> bool {
    let archive_name = Path::new(name).file_name().and_then(|s| s.to_str());
    sanitize_filename::is_sanitized(name)
        && archive_name.is_some_and(|f| f == name)
        && name.find(['/', '\\']).is_none()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ServerTlsConfig {
    pub(crate) server_cert: PathBuf,
    pub(crate) server_key: PathBuf,
    pub(crate) root_ca: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ServerConfig {
    // 一旦为 Some, 自动启用 tls.
    #[serde(default, rename = "tls")]
    pub(crate) tls_config: Option<ServerTlsConfig>,
    #[serde(default = "default_bind_address", rename = "bind")]
    pub(crate) bind_address: SocketAddr,
}

impl Default for ServerConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

fn default_bind_address() -> SocketAddr {
    "0.0.0.0:20531".parse().unwrap()
}

impl ServerConfig {
    pub(crate) async fn from_toml_file(
        file: impl AsRef<Path>,
        create_new: bool,
    ) -> crate::Result<Self> {
        let config_path = file.as_ref();
        let content = fs::read_to_string(&config_path).await;
        match content {
            Ok(content) => Ok(toml::from_str(&content)?),
            Err(e) => {
                if !create_new {
                    Err(e)?;
                }
                let default_config = Self::default();
                fs::write(
                    config_path,
                    toml::to_string_pretty(&default_config)?.as_bytes(),
                )
                .await?;
                Ok(default_config)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct RoomConfig {
    pub room_no: String,
    pub elcarea: i32,
    pub elcbuis: String,
}

impl RoomConfig {
    #[must_use]
    pub fn empty() -> Self {
        RoomConfig::default()
    }

    #[inline]
    #[must_use]
    pub fn is_invalid(&self) -> bool {
        self.elcarea < 0 || self.room_no.is_empty() || self.elcbuis.is_empty()
    }

    pub(crate) fn dir(&self) -> crate::Result<PathBuf> {
        let room_dir = data_dir()?.join(ROOMS_DIRNAME);
        let room_no_dir = room_dir.join(match self.room_no.as_str() {
            s if s.trim().is_empty() => ROOM_UNKNOWN_DIRNAME,
            s if !is_sanitized_filename(s) => Err(CSError::InvalidRoomConfig)?,
            s => s,
        });
        std::fs::create_dir_all(&room_no_dir)?;
        Ok(room_no_dir)
    }

    pub async fn from_toml_file(room_config_file: impl AsRef<Path>) -> crate::Result<Self> {
        let config_path = room_config_file.as_ref();
        let room_config: Self = toml::from_str(
            &fs::read_to_string(&config_path)
                .await
                .map_err(|e| Error::FileRead(config_path.into(), e.to_string()))?,
        )?;
        Ok(room_config)
    }

    pub async fn save_to_file(&self, file: impl AsRef<Path>) -> crate::Result<()> {
        fs::write(file, toml::to_string(self)?).await?;
        Ok(())
    }
}

pub(crate) fn data_dir() -> io::Result<PathBuf> {
    let default_data_dir = shellexpand::tilde("~/.local/share");
    let data_dir = dirs_next::data_dir()
        .unwrap_or(default_data_dir.to_string().into())
        .join(PKG_NAME);
    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir)
}

pub(crate) fn config_dir() -> io::Result<PathBuf> {
    let default_config_dir = shellexpand::tilde("~/.config");
    let config_dir = dirs_next::config_dir()
        .unwrap_or(default_config_dir.to_string().into())
        .join(PKG_NAME);
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

pub(crate) fn log_dir() -> io::Result<PathBuf> {
    let log_dir = data_dir()?.join(LOG_DIRNAME);
    std::fs::create_dir_all(&log_dir)?;
    Ok(log_dir)
}

pub(crate) const PKG_NAME: &str = if cfg!(debug_assertions) {
    concat!(env!("CARGO_PKG_NAME"), "-debug")
} else {
    env!("CARGO_PKG_NAME")
};
pub(crate) const RECORDS_FILENAME: &str = "records.csv";
pub(crate) const ARCHIVE_DIRNAME: &str = "archives";
pub(crate) const ROOM_CONFIG_FILENAME: &str = "room.toml";
pub(crate) const LOG_DIRNAME: &str = "logs";
pub(crate) const SERVER_CONFIG_FILENAME: &str = "server.toml";
pub(crate) const DELETED_DIRNAME: &str = "deleted";
pub(crate) const ROOMS_DIRNAME: &str = "rooms";
pub(crate) const ROOM_UNKNOWN_DIRNAME: &str = "unknown";
