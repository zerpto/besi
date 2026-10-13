//! PostgreSQL connection pool (feature `db`).
//!
//! One pool per process, created lazily by the runner from the `database`
//! config section: no connection is opened until the first query, so
//! commands that never touch the database work without Postgres running.
//! Commands reach it with `ctx.db()`, HTTP handlers with `state.db()`.
//!
//! With feature `orm`, the same pool is also a SeaORM connection
//! (`db.orm()`), and `MigrateCommand` applies the app's migrations.

#[cfg(feature = "orm")]
mod migrate;
mod pool;
mod settings;

#[cfg(feature = "orm")]
pub use migrate::MigrateCommand;
pub use pool::Db;
pub use settings::DatabaseConfig;
