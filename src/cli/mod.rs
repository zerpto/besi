//! Running the application: the top-level `App`, the `Command` trait,
//! the `Context` passed to commands, and the `run` entry point.

mod app;
mod command;
mod context;
mod runner;

pub use app::App;
pub use command::Command;
pub use context::Context;
pub use runner::run;
