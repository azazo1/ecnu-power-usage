use clap::Parser;
use ecnu_power_usage::{
    config::{self, load_room_config},
    server::Querier,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct AppArgs {
    #[clap(short, long, help="config dir path", default_value = config::DEFAULT_CONFIG_DIR)]
    config: String,
}

struct App {
    args: AppArgs,
}

impl App {
    fn new(args: AppArgs) -> Self {
        Self { args }
    }

    async fn run(self) -> anyhow::Result<()> {
        let room_config = load_room_config(&self.args.config)?;
        let querier = Querier::new(room_config);
        dbg!(querier.query_electricity_balance().await)?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = AppArgs::parse();
    App::new(args).run().await
}
