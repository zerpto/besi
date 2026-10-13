use crate::{Context, Result};

/// A unit of work the application can execute.
///
/// Implement this for each subcommand's argument struct, and for the
/// top-level CLI type (which dispatches to the selected subcommand).
/// Commands are async so they can use async resources (database, HTTP)
/// directly; the runner drives them on a tokio runtime.
// `async fn` in a public trait can't promise `Send` futures. That's fine
// here: the runner `block_on`s the command on its own thread, never spawns it.
#[allow(async_fn_in_trait)]
pub trait Command {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()>;
}
