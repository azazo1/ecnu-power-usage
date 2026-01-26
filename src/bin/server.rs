use clap::Parser;
use ecnu_power_usage::server::run_app;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct AppArgs {
    #[clap(
        short,
        long,
        help = "service bind address",
        default_value = "0.0.0.0:20531"
    )]
    bind: SocketAddr,
}

struct App {
    args: AppArgs,
}

impl App {
    fn new(args: AppArgs) -> Self {
        Self { args }
    }

    async fn run(self) -> anyhow::Result<()> {
        run_app(self.args.bind).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = AppArgs::parse();
    App::new(args).run().await
}
