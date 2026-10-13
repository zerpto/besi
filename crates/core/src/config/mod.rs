//! Layered configuration.
//!
//! Sources, lowest to highest priority (later ones override earlier ones):
//!
//! 1. YAML file (`config.yaml` by default)
//! 2. `.env` file (read as key/value pairs; does not modify the process env)
//! 3. Process environment variables
//!
//! Env and `.env` keys map to config keys by lowercasing and splitting on
//! `__`: `DATABASE__MAX_CONN=10` sets `database.max_conn`.
//!
//! Modules read their own part of the config through [`ConfigSection`], so
//! the core never needs to know which sections exist.
//!
//! Note: the `config` dependency shares this module's name, so it is always
//! written `::config::...` inside the crate.

mod args;
mod loader;
mod section;
mod sources;
mod store;

pub use args::ConfigArgs;
pub use loader::ConfigLoader;
pub use section::ConfigSection;
pub use store::Config;
