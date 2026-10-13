use std::io;
use std::process::ExitCode;

use crate::{App, Context, Result};

/// Parse arguments into `A`, load config, run it against stdout, and map
/// the result to a process exit code. Call this from the application's `main`.
///
/// Owns the process's single tokio runtime: every command runs on it.
pub fn run<A: App>() -> ExitCode {
    let app = A::parse();
    let result = tokio::runtime::Runtime::new()
        .map_err(Into::into)
        .and_then(|runtime| runtime.block_on(try_run(&app)));

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn try_run<A: App>(app: &A) -> Result<()> {
    let config = app.config_loader().load()?;

    // Not `.lock()`: holding the lock for the whole run would deadlock any
    // other thread that writes to stdout (e.g. async workers in `api`).
    let mut stdout = io::stdout();
    let ctx = Context::new(&mut stdout, &config);

    #[cfg(feature = "db")]
    let db = crate::Db::connect_lazy(&config.section()?)?;
    #[cfg(feature = "db")]
    let ctx = ctx.with_db(db.clone());

    let mut ctx = ctx;
    let result = app.run(&mut ctx).await;

    #[cfg(feature = "db")]
    db.close().await;

    result
}
