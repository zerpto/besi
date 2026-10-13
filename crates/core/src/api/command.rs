use std::marker::PhantomData;

use clap::Args;

use super::{ApiConfig, ApiRoutes, ApiState, server};
use crate::{Command, Context, Result};

/// The `api` subcommand: start the HTTP server on `R`'s routes.
///
/// Runs until Ctrl+C or SIGTERM. Like every other command, it is dispatched
/// with plain [`Command::run`]; the app's routes arrive through the type
/// parameter, not through a special method.
#[derive(Debug, Args)]
pub struct ApiCommand<R: ApiRoutes> {
    /// Address to bind [default: api.host from config, or 127.0.0.1]
    #[arg(long)]
    host: Option<String>,

    /// Port to listen on [default: api.port from config, or 8080]
    #[arg(long)]
    port: Option<u16>,

    #[arg(skip)]
    routes: PhantomData<fn() -> R>,
}

impl<R: ApiRoutes> Command for ApiCommand<R> {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        let settings = self.settings(ctx)?;
        let app = server::router::<R>(Self::state(ctx));

        crate::daemon::init_logging();
        server::serve(&settings, app).await
    }
}

impl<R: ApiRoutes> ApiCommand<R> {
    /// Handler state: the command's config, plus its database pool if any.
    fn state(ctx: &Context<'_>) -> ApiState {
        let state = ApiState::new(ctx.config().clone());
        #[cfg(feature = "db")]
        let state = match ctx.db() {
            Ok(db) => state.with_db(db.clone()),
            Err(_) => state,
        };
        state
    }

    /// Config section, overridden by CLI flags.
    fn settings(&self, ctx: &Context<'_>) -> Result<ApiConfig> {
        let mut settings: ApiConfig = ctx.config().section()?;
        if let Some(host) = &self.host {
            settings.host = host.clone();
        }
        if let Some(port) = self.port {
            settings.port = port;
        }
        Ok(settings)
    }
}
