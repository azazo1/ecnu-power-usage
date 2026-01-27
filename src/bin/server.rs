use clap::Parser;
use ecnu_power_usage::server::run_app;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct AppArgs {}

#[allow(dead_code)]
struct App {
    args: AppArgs,
}

impl App {
    fn new(args: AppArgs) -> Self {
        Self { args }
    }

    async fn run(self) -> anyhow::Result<()> {
        run_app().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = AppArgs::parse();
    App::new(args).run().await
}
