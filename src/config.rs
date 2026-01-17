use anyhow::Context;

use crate::error::{Error, Result};
use std::{fs, ops::Deref, path::Path};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomConfig {
    pub room_no: String,
    pub elcarea: i32,
    pub elcbuis: String,
}

pub const DEFAULT_CONFIG_DIR: &str = "~/.config/ecnu-power-usage/";

pub fn load_room_config(config_dir: impl AsRef<str>) -> Result<RoomConfig> {
    let config_dir = config_dir.as_ref();
    let config_dir = shellexpand::tilde(config_dir);
    // fs::create_dir_all(config_dir.deref())?;
    let config_path = Path::new(config_dir.deref()).join("room.toml");
    let room_config: RoomConfig = toml::from_str(
        &fs::read_to_string(&config_path)
            .map_err(|e| Error::ConfigFileReadError(config_path, e.to_string()))?,
    )?;
    Ok(room_config)
}
