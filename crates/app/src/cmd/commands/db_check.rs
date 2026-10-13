use besi::{Command, Context, Result};
use clap::Args;

/// Connect to the configured database and print the server version.
#[derive(Debug, Args)]
pub struct DbCheckCommand {}

impl Command for DbCheckCommand {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        let version: String = besi::sqlx::query_scalar("SELECT version()")
            .fetch_one(ctx.db()?.pool())
            .await?;
        writeln!(ctx.out(), "connected: {version}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use besi::{Config, DatabaseConfig, Db};

    use super::*;

    #[tokio::test]
    async fn reports_error_when_database_unreachable() {
        let db = Db::connect_lazy(&DatabaseConfig {
            host: "127.0.0.1".into(),
            port: 1,
            acquire_timeout_secs: 1,
            ..DatabaseConfig::default()
        })
        .unwrap();

        let mut buf = Vec::new();
        let config = Config::default();
        let mut ctx = Context::new(&mut buf, &config).with_db(db);

        let err = DbCheckCommand {}.run(&mut ctx).await.unwrap_err();
        assert!(err.to_string().starts_with("database error"), "{err}");
        assert!(buf.is_empty());
    }
}
