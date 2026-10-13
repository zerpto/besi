//! Core framework for building console commands.
//!
//! This crate knows nothing about concrete commands. Applications define
//! their commands (with clap), implement [`Command`] for them, and hand the
//! top-level parser to [`run`]. Configuration is loaded once at startup and
//! exposed to commands through [`Context::config`].
//!
//! Optional features:
//!
//! - `api`: HTTP API server and the `api` command ([`ApiCommand`]).
//! - `db`: PostgreSQL connection pool ([`Db`]), as `ctx.db()` / `state.db()`.
//! - `cron`: job scheduler and the `cron` command ([`CronCommand`]).
//! - `orm`: SeaORM on the `db` pool (`db.orm()`) and the `migrate` command
//!   ([`MigrateCommand`]). Implies `db`.
//!
//! Internals are organized by concept (`cli`, `config`, `error`, `api`, `db`,
//! `cron`, `daemon`); everything
//! public is re-exported here, so callers always write `besi::Item`.

#[cfg(feature = "api")]
mod api;
mod cli;
mod config;
#[cfg(feature = "cron")]
mod cron;
#[cfg(any(feature = "api", feature = "cron"))]
mod daemon;
#[cfg(feature = "db")]
mod db;
mod error;

pub use cli::{App, Command, Context, run};
pub use config::{Config, ConfigArgs, ConfigLoader, ConfigSection};
pub use error::{Error, Result};

#[cfg(feature = "api")]
pub use api::{ApiCommand, ApiConfig, ApiRoutes, ApiState, Health};
/// Re-exported so apps write routes against the same axum version as the core.
#[cfg(feature = "api")]
pub use axum;

#[cfg(feature = "cron")]
pub use cron::{CronCommand, CronConfig, CronJobs, Job, JobContext, JobSettings};

#[cfg(feature = "db")]
pub use db::{DatabaseConfig, Db};
/// Re-exported so apps write queries against the same sqlx version as the core.
#[cfg(feature = "db")]
pub use sqlx;

#[cfg(feature = "orm")]
pub use db::MigrateCommand;
/// Re-exported so apps query against the same SeaORM version as the core.
#[cfg(feature = "orm")]
pub use sea_orm;
