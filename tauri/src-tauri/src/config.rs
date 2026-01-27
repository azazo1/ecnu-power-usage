use std::path::{Path, PathBuf};

use ecnu_power_usage::client::Client;
use reqwest::{Certificate, Identity};
use serde::{Deserialize, Serialize};
use tauri::{Url, async_runtime::RwLock};
use tokio::{
    fs,
    io::{self, AsyncReadExt, AsyncWriteExt},
};
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
#[serde(rename_all = "camelCase")]
pub(crate) struct GuiConfig {
    #[serde(default = "default_server_base")]
    server_base: Url,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_cert: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_key: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "rootCA")]
    root_ca: Option<PathBuf>,
    #[serde(default)]
    use_self_signed_tls: bool,
}

impl Default for GuiConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

fn default_server_base() -> Url {
    "https://localhost:20531".parse().unwrap()
}

impl GuiConfig {
    pub(crate) async fn load_config() -> crate::Result<Self> {
        let config_path = config_dir().await?.join(CONFIG_FILENAME);
        info!("loading config: {config_path:?}");
        let config = toml::from_str(&fs::read_to_string(config_path).await.unwrap_or_else(|e| {
            warn!("read config file error: {e:?}, use default config instead.");
            String::new()
        }))?;
        info!("config loaded: {config:#?}");
        Ok(config)
    }

    pub(crate) async fn save_config(&self) -> crate::Result<()> {
        let config_path = config_dir().await?.join(CONFIG_FILENAME);
        Ok(fs::write(config_path, toml::to_string_pretty(self)?.as_bytes()).await?)
    }
}

/// 验证证书和密钥内容, 导入到内部路径, 返回的内部路径和输入参数顺序一致, 且都是全局路径.
async fn import_tls(
    client_crt: &Path,
    client_key: &Path,
    root_ca: &Path,
) -> crate::Result<(PathBuf, PathBuf, PathBuf)> {
    async fn checked_read(p: &Path) -> crate::Result<String> {
        const MAX_LEN: u64 = 50 * 1024;
        let meta = fs::metadata(p).await?;
        if meta.len() > MAX_LEN {
            return Err(crate::Error::CertFileTooLarge);
        }
        if !meta.is_file() {
            return Err(crate::Error::CertNotFile);
        }
        let mut content = String::with_capacity(meta.len() as usize);
        let file = fs::File::options().read(true).open(p).await?;
        file.take(MAX_LEN).read_to_string(&mut content).await?;
        Ok(content)
    }
    let client_crt = checked_read(client_crt).await?;
    let client_key = checked_read(client_key).await?;
    let root_ca = checked_read(root_ca).await?;

    // 测试是否是有效的证书和密钥.
    Certificate::from_pem(root_ca.as_ref())?;
    Identity::from_pem(format!("{client_crt}\n{client_key}").as_ref())?;

    let tls_dir = data_dir().await?.join(TLS_DIRNAME);
    fs::create_dir_all(&tls_dir).await?;
    let client_crt_path = tls_dir.join(CLIENT_CERT_FILENAME);
    let client_key_path = tls_dir.join(CLIENT_KEY_FILENAME);
    let root_ca_path = tls_dir.join(ROOT_CA_FILENAME);

    let mut options = fs::File::options();
    options.write(true).truncate(true).create(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }

    options
        .open(&client_crt_path)
        .await?
        .write_all(client_crt.as_ref())
        .await?;
    options
        .open(&client_key_path)
        .await?
        .write_all(client_key.as_ref())
        .await?;
    options
        .open(&root_ca_path)
        .await?
        .write_all(root_ca.as_ref())
        .await?;

    Ok((client_crt_path, client_key_path, root_ca_path))
}

pub(crate) struct AppState {
    pub(crate) config: RwLock<GuiConfig>,
    pub(crate) client: RwLock<Client>,
}

impl AppState {
    pub(crate) async fn load() -> crate::Result<Self> {
        let config = GuiConfig::load_config().await?;
        let this = Self {
            client: RwLock::new(Client::new(config.server_base.clone())),
            config: RwLock::new(GuiConfig::default()),
        };
        this.set_config(config).await?;
        Ok(this)
    }

    pub(crate) async fn set_config(&self, mut new_config: GuiConfig) -> crate::Result<()> {
        let mut client = self.client.write().await;
        if new_config.use_self_signed_tls
            && let Some(client_cert) = new_config.client_cert.as_ref()
            && let Some(client_key) = new_config.client_key.as_ref()
            && let Some(root_ca) = new_config.root_ca.as_ref()
        {
            // 这三个都是 path, 验证证书内容并复制到内部路径.
            let (client_cert, client_key, root_ca) =
                import_tls(client_cert.as_ref(), client_key.as_ref(), root_ca.as_ref()).await?;
            client
                .configure_tls(&client_cert, &client_key, &root_ca)
                .await?;
            new_config.client_cert = Some(client_cert);
            new_config.client_key = Some(client_key);
            new_config.root_ca = Some(root_ca);
        } else {
            client.deconfigure_tls();
        }
        client.set_server_base(new_config.server_base.clone());
        drop(client);

        let mut config = self.config.write().await;
        *config = new_config;
        config.save_config().await?;
        drop(config);
        Ok(())
    }
}

pub(crate) const CONFIG_FILENAME: &str = "gui-config.toml";
pub(crate) const PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const ARCHIVE_CACHE_DIRNAME: &str = "archive-cache";
pub(crate) const TLS_DIRNAME: &str = "tls";
pub(crate) const CLIENT_CERT_FILENAME: &str = "client.crt";
pub(crate) const CLIENT_KEY_FILENAME: &str = "client.key";
pub(crate) const ROOT_CA_FILENAME: &str = "root-ca.crt";
