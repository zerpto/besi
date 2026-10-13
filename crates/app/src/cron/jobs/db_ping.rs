use besi::{JobContext, Result};

/// Check the database answers. A failure is logged by the scheduler, so a
/// schedule like `*/30 * * * * *` gives a running record of DB reachability.
pub async fn run(ctx: JobContext) -> Result<()> {
    ctx.db()?.ping().await
}

#[cfg(test)]
mod tests {
    use besi::{Config, DatabaseConfig, Db};

    use super::*;

    #[tokio::test]
    async fn fails_when_database_unreachable() {
        let db = Db::connect_lazy(&DatabaseConfig {
            host: "127.0.0.1".into(),
            port: 1,
            ..DatabaseConfig::default()
        })
        .unwrap();
        let ctx = JobContext::new("db_ping", Config::default()).with_db(db);
        assert!(run(ctx).await.is_err());
    }
}
