use chromiumoxide::browser::BrowserConfig;
use ecnu_power_usage::{client::BrowserExecutor, error::Error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let be = BrowserExecutor::new(
        BrowserConfig::builder()
            .with_head()
            .build()
            .map_err(Error::ChromiumParamBuildingError)?,
    )
    .await?;
    be.pick_room().await?;
    be.close().await?;
    Ok(())
}
