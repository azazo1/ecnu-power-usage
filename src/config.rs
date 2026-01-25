use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

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

    pub async fn from_file(room_config_file: impl AsRef<Path>) -> Result<RoomConfig> {
        let config_path = room_config_file.as_ref();
        let room_config: RoomConfig = toml::from_str(
            &fs::read_to_string(&config_path)
                .await
                .map_err(|e| Error::FileRead(config_path.into(), e.to_string()))?,
        )?;
        Ok(room_config)
    }

    pub async fn save_to_file(&self, file: impl AsRef<Path>) -> Result<()> {
        fs::write(file, toml::to_string(self)?).await?;
        Ok(())
    }
}

pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const RECORDS_FILENAME: &str = "records.csv";
pub const ARCHIVE_DIRNAME: &str = "archives";
pub const ROOM_CONFIG_FILENAME: &str = "room.toml";
