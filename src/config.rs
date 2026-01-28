use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};
use tokio::fs;

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
                    Err(e)?
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RoomConfig {
    pub room_no: String,
    pub elcarea: i32,
    pub elcbuis: String,
}

impl RoomConfig {
    pub fn is_invalid(&self) -> bool {
        self.elcarea < 0 || self.room_no.is_empty() || self.elcbuis.is_empty()
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

pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const RECORDS_FILENAME: &str = "records.csv";
pub const ARCHIVE_DIRNAME: &str = "archives";
pub const ROOM_CONFIG_FILENAME: &str = "room.toml";
pub const LOG_DIRNAME: &str = "logs";
pub(crate) const SERVER_CONFIG_FILE: &str = "server.toml";
