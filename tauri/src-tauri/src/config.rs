use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use ecnu_power_usage::client::Client;
use serde::{Deserialize, Serialize};
use tauri::{async_runtime::RwLock, Url};
use tokio::{fs, io};
use tracing::{info, warn};

pub(crate) async fn log_dir() -> crate::Result<PathBuf> {
    let default = shellexpand::tilde("~/.local/share");
    let dir = dirs_next::data_dir()
        .unwrap_or(default.to_string().into())
        .join(PKG_NAME)
        .join("logs");
    fs::create_dir_all(&dir).await?;
    Ok(dir)
}

pub(crate) async fn data_dir() -> crate::Result<PathBuf> {
    let default = shellexpand::tilde("~/.local/share");
    let dir = dirs_next::data_dir()
        .unwrap_or(default.to_string().into())
        .join(PKG_NAME);
    fs::create_dir_all(&dir).await?;
    Ok(dir)
}

pub(crate) async fn config_dir() -> io::Result<PathBuf> {
    let default = shellexpand::tilde("~/.config");
    let dir = dirs_next::config_dir()
        .unwrap_or(default.to_string().into())
        .join(PKG_NAME);
    fs::create_dir_all(&dir).await?;
    Ok(dir)
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct GuiConfig {
    server_base: Url,
}

impl GuiConfig {
    pub(crate) async fn load_config() -> crate::Result<Self> {
        let config_path = config_dir().await?.join(CONFIG_FILENAME);
        info!("loading config: {config_path:?}");
        let config = toml::from_str(&fs::read_to_string(config_path).await?)?;
        info!("config loaded.");
        Ok(config)
    }

    pub(crate) async fn save_config(&self) -> crate::Result<()> {
        let config_path = config_dir().await?.join(CONFIG_FILENAME);
        Ok(fs::write(config_path, toml::to_string_pretty(self)?.as_bytes()).await?)
    }

    fn set_server_base(&mut self, server_base: Url) -> ConfigSync {
        self.server_base = server_base;
        ConfigSync::new(self)
    }
}

/// 延迟, 将配置统一地写入到文件当中.
struct ConfigSync<'a> {
    config: &'a mut GuiConfig,
    sync: bool,
}

impl<'a> ConfigSync<'a> {
    fn new(config: &'a mut GuiConfig) -> Self {
        Self {
            config,
            sync: false,
        }
    }

    async fn sync_to_file(mut self) -> crate::Result<()> {
        self.config.save_config().await?;
        self.sync = true;
        Ok(())
    }
}

impl<'a> Deref for ConfigSync<'a> {
    type Target = GuiConfig;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl<'a> DerefMut for ConfigSync<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}

impl<'a> Drop for ConfigSync<'a> {
    fn drop(&mut self) {
        if !self.sync {
            warn!("ConfigSycn was dropped before sync.");
        }
    }
}

pub(crate) struct AppState {
    config: RwLock<GuiConfig>,
    client: RwLock<Client>,
}

impl AppState {
    pub(crate) async fn load() -> crate::Result<Self> {
        let config = GuiConfig::load_config().await?;
        Ok(Self {
            client: RwLock::new(Client::new(config.server_base.clone())),
            config: RwLock::new(config),
        })
    }

    pub(crate) async fn set_server_base(&self, server_base: Url) -> crate::Result<()> {
        self.client
            .write()
            .await
            .set_server_base(server_base.clone());
        self.config
            .write()
            .await
            .set_server_base(server_base)
            .sync_to_file()
            .await?;
        Ok(())
    }

    pub(crate) async fn set_config(&self, new_config: GuiConfig) -> crate::Result<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        config.save_config().await
    }
}

pub(crate) const CONFIG_FILENAME: &str = "gui-config.toml";
pub(crate) const PKG_NAME: &str = env!("CARGO_PKG_NAME");
