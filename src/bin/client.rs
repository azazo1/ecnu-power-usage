use ecnu_power_usage::client::GuardClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let client = GuardClient::new("http://localhost:20531".parse()?);
    client.guard().await?;
    Ok(())
}
