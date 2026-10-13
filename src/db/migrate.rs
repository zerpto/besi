use std::marker::PhantomData;

use clap::{Args, Subcommand};
use sea_orm_migration::MigratorTrait;

use super::DatabaseConfig;
use crate::{Command, Context, Error, Result};

/// The `migrate` subcommand, on migrator `M` (the app's migration crate).
///
/// Like every other command, dispatched with plain [`Command::run`]; the
/// app's migrations arrive through the type parameter:
/// `Migrate(besi::MigrateCommand<migration::Migrator>)`.
#[derive(Debug, Args)]
pub struct MigrateCommand<M: MigratorTrait> {
    #[command(subcommand)]
    action: MigrateAction,

    #[arg(skip)]
    migrator: PhantomData<fn() -> M>,
}

#[derive(Debug, Subcommand)]
enum MigrateAction {
    /// List every migration and whether it is applied
    Status,
    /// Apply pending migrations (all, or the next --steps N)
    Up {
        /// Apply at most this many
        #[arg(long)]
        steps: Option<u32>,
    },
    /// Roll back the most recent migration (or the last --steps N)
    Down {
        /// Roll back this many
        #[arg(long, default_value_t = 1)]
        steps: u32,
    },
    /// DROP EVERY TABLE in the database, then apply all migrations
    Fresh {
        /// Required: confirms dropping every table
        #[arg(long)]
        yes: bool,
    },
}

impl<M: MigratorTrait> Command for MigrateCommand<M> {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        // Refuse before touching the database.
        if let MigrateAction::Fresh { yes: false } = self.action {
            let name = ctx.config().section::<DatabaseConfig>()?.name;
            return Err(Error::msg(format!(
                "`migrate fresh` drops every table in database {name:?}; re-run with --yes to confirm"
            )));
        }

        let db = ctx.db()?.orm().clone();
        match &self.action {
            MigrateAction::Status => {
                let all = M::get_migration_with_status(&db).await?;
                if all.is_empty() {
                    writeln!(ctx.out(), "no migrations defined")?;
                }
                let width = all.iter().map(|m| m.name().len()).max().unwrap_or(0);
                for migration in all {
                    writeln!(
                        ctx.out(),
                        "{:width$}  {}",
                        migration.name(),
                        migration.status()
                    )?;
                }
            }
            MigrateAction::Up { steps } => {
                let pending = M::get_pending_migrations(&db).await?;
                if pending.is_empty() {
                    writeln!(ctx.out(), "nothing to apply")?;
                    return Ok(());
                }
                M::up(&db, *steps).await?;
                let count = steps.map_or(pending.len(), |n| (n as usize).min(pending.len()));
                for migration in &pending[..count] {
                    writeln!(ctx.out(), "applied      {}", migration.name())?;
                }
            }
            MigrateAction::Down { steps } => {
                let applied = M::get_applied_migrations(&db).await?;
                if applied.is_empty() {
                    writeln!(ctx.out(), "nothing to roll back")?;
                    return Ok(());
                }
                M::down(&db, Some(*steps)).await?;
                let count = (*steps as usize).min(applied.len());
                for migration in applied.iter().rev().take(count) {
                    writeln!(ctx.out(), "rolled back  {}", migration.name())?;
                }
            }
            MigrateAction::Fresh { .. } => {
                M::fresh(&db).await?;
                let count = M::get_applied_migrations(&db).await?.len();
                writeln!(
                    ctx.out(),
                    "dropped all tables, applied {count} migration(s)"
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sea_orm_migration::MigrationTrait;

    use super::*;
    use crate::{Config, Db};

    #[derive(Debug)]
    struct NoMigrations;

    #[async_trait::async_trait]
    impl MigratorTrait for NoMigrations {
        fn migrations() -> Vec<Box<dyn MigrationTrait>> {
            Vec::new()
        }
    }

    fn command(action: MigrateAction) -> MigrateCommand<NoMigrations> {
        MigrateCommand {
            action,
            migrator: PhantomData,
        }
    }

    fn unreachable() -> Db {
        Db::connect_lazy(&DatabaseConfig {
            host: "127.0.0.1".into(),
            port: 1,
            acquire_timeout_secs: 1,
            ..DatabaseConfig::default()
        })
        .unwrap()
    }

    #[tokio::test]
    async fn fresh_requires_yes_and_never_connects() {
        let (mut buf, config) = (Vec::new(), Config::default());
        // No database attached at all: the refusal must come first.
        let mut ctx = Context::new(&mut buf, &config);
        let err = command(MigrateAction::Fresh { yes: false })
            .run(&mut ctx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("re-run with --yes"), "{err}");
        assert!(err.to_string().contains("\"postgres\""), "{err}");
    }

    #[tokio::test]
    async fn status_reports_unreachable_database() {
        let (mut buf, config) = (Vec::new(), Config::default());
        let mut ctx = Context::new(&mut buf, &config).with_db(unreachable());
        let err = command(MigrateAction::Status)
            .run(&mut ctx)
            .await
            .unwrap_err();
        assert!(err.to_string().starts_with("database error"), "{err}");
        assert!(buf.is_empty());
    }
}
