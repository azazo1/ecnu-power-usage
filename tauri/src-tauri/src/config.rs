use std::path::PathBuf;

use ecnu_power_usage::client::Client;
use serde::{Deserialize, Serialize};
use tauri::{Url, async_runtime::RwLock};
use tokio::{fs, io};
use tracing::info;

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
#[serde(rename_all = "camelCase")]
pub(crate) struct GuiConfig {
    #[serde(default = "default_server_base")]
    server_base: Url,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_cert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "rootCA")]
    root_ca: Option<String>,
    #[serde(default)]
    use_self_signed_tls: bool,
}

fn default_server_base() -> Url {
    "https://localhost:20531".parse().unwrap()
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
}

pub(crate) struct AppState {
    pub(crate) config: RwLock<GuiConfig>,
    pub(crate) client: RwLock<Client>,
}

impl AppState {
    pub(crate) async fn load() -> crate::Result<Self> {
        let config = GuiConfig::load_config().await?;
        Ok(Self {
            client: RwLock::new(Client::new(config.server_base.clone())),
            config: RwLock::new(config),
        })
    }

    pub(crate) async fn set_config(&self, new_config: GuiConfig) -> crate::Result<()> {
        let mut config = self.config.write().await;
        *config = new_config.clone();
        config.save_config().await?;
        drop(config);

        let mut client = self.client.write().await;
        client.set_server_base(new_config.server_base);
        if new_config.use_self_signed_tls
            && let Some(client_cert) = new_config.client_cert
            && let Some(client_key) = new_config.client_key
            && let Some(root_ca) = new_config.root_ca
        {
            client.configure_tls(client_cert, client_key, root_ca);
        } else {
            client.deconfigure_tls();
        }
        Ok(())
    }
}

pub(crate) const CONFIG_FILENAME: &str = "gui-config.toml";
pub(crate) const PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const ARCHIVE_CACHE_DIRNAME: &str = "archive-cache";
