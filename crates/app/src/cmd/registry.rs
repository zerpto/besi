//! The app's command list: one `Commands` variant per subcommand, and the
//! dispatch that runs the selected one.
//!
//! To add a command: create `commands/<name>.rs` with an args struct that
//! implements `Command`, declare it in `commands/mod.rs`, then add a variant
//! and a match arm here.

use besi::{Command, Context, Result};
use clap::Subcommand;

use super::commands::{db_check, hello};
use crate::api::AppRoutes;
use crate::cron::AppJobs;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Print a greeting
    Hello(hello::HelloCommand),
    /// Start the HTTP API server
    Api(besi::ApiCommand<AppRoutes>),
    /// Check the database connection and print the server version
    DbCheck(db_check::DbCheckCommand),
    /// Run scheduled jobs (`--list` to show them, `run <job>` to run one now)
    Cron(besi::CronCommand<AppJobs>),
    /// Manage database migrations (status, up, down, fresh)
    Migrate(besi::MigrateCommand<migration::Migrator>),
}

impl Command for Commands {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        match self {
            Self::Hello(cmd) => cmd.run(ctx).await,
            Self::Api(cmd) => cmd.run(ctx).await,
            Self::DbCheck(cmd) => cmd.run(ctx).await,
            Self::Cron(cmd) => cmd.run(ctx).await,
            Self::Migrate(cmd) => cmd.run(ctx).await,
        }
    }
}
