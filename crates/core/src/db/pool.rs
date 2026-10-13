use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use super::DatabaseConfig;
use crate::{Error, Result};

/// Upper bound for [`Db::ping`], kept under the API's 2 s health timeout so
/// a down database reports "unreachable" rather than "timed out".
const PING_TIMEOUT: Duration = Duration::from_secs(1);

/// The application's PostgreSQL pool. Cheap to clone (clones share the
/// pool). Run raw sqlx queries against [`Db::pool`], or SeaORM queries
/// against [`Db::orm`] (feature `orm`); both use the same connections:
///
/// ```ignore
/// let names: Vec<String> = sqlx::query_scalar("select name from users")
///     .fetch_all(ctx.db()?.pool())
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct Db {
    pool: PgPool,
    /// SeaORM connection wrapping `pool`, not a second pool.
    #[cfg(feature = "orm")]
    orm: sea_orm::DatabaseConnection,
}

impl Db {
    /// Build the pool without connecting; connections open on first use.
    /// Must be called inside a tokio runtime.
    pub fn connect_lazy(config: &DatabaseConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout())
            .connect_lazy_with(config.connect_options()?);
        Ok(Self {
            #[cfg(feature = "orm")]
            orm: sea_orm::SqlxPostgresConnector::from_sqlx_postgres_pool(pool.clone()),
            pool,
        })
    }

    /// The sqlx pool, for `query(...).fetch_*(db.pool())`.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// SeaORM connection on the same pool, e.g.
    /// `entity::note::Entity::find().all(db.orm()).await?`.
    #[cfg(feature = "orm")]
    pub fn orm(&self) -> &sea_orm::DatabaseConnection {
        &self.orm
    }

    /// Round-trip `SELECT 1`, bounded by a short timeout. For health checks.
    pub async fn ping(&self) -> Result<()> {
        tokio::time::timeout(PING_TIMEOUT, sqlx::query("SELECT 1").execute(&self.pool))
            .await
            .map_err(|_| Error::msg("database ping timed out"))??;
        Ok(())
    }

    /// Close all connections, waiting for checked-out ones to be returned.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing listens on port 1, so connecting fails fast.
    fn unreachable() -> DatabaseConfig {
        DatabaseConfig {
            host: "127.0.0.1".into(),
            port: 1,
            acquire_timeout_secs: 30,
            ..DatabaseConfig::default()
        }
    }

    #[tokio::test]
    async fn connect_lazy_does_not_connect() {
        // Would fail if it tried to reach port 1.
        Db::connect_lazy(&unreachable()).unwrap();
    }

    #[tokio::test]
    async fn ping_fails_within_timeout_when_unreachable() {
        let db = Db::connect_lazy(&unreachable()).unwrap();
        let started = std::time::Instant::now();
        assert!(db.ping().await.is_err());
        // Bounded by PING_TIMEOUT, not the 30 s acquire timeout.
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "{:?}",
            started.elapsed()
        );
    }
}
