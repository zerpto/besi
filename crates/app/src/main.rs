mod api;
mod cmd;
mod cron;

use std::process::ExitCode;

use besi::{App, Command, ConfigArgs, ConfigLoader, Context, Result};
use clap::Parser;

use cmd::Commands;

#[derive(Debug, Parser)]
#[command(name = env!("CARGO_BIN_NAME"), version, about)]
struct Cli {
    #[command(flatten)]
    config: ConfigArgs,

    #[command(subcommand)]
    command: Commands,
}

impl Command for Cli {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        self.command.run(ctx).await
    }
}

impl App for Cli {
    fn config_loader(&self) -> ConfigLoader {
        self.config.loader()
    }
}

fn main() -> ExitCode {
    besi::run::<Cli>()
}
