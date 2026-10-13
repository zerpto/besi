//! CLI side of the app: `commands/` holds one module per subcommand,
//! `registry.rs` lists and dispatches them.

pub mod commands;
mod registry;

pub use registry::Commands;
