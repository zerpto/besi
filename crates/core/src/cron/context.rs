use std::sync::Arc;

use crate::{Config, Context};
#[cfg(feature = "db")]
use crate::{Db, Error, Result};

/// What a running job receives: its name, the config, and the database.
/// Cheap to clone; every run gets its own.
#[derive(Debug, Clone)]
pub struct JobContext {
    name: &'static str,
    config: Arc<Config>,
    #[cfg(feature = "db")]
    db: Option<Db>,
}

impl JobContext {
    pub fn new(name: &'static str, config: Config) -> Self {
        Self {
            name,
            config: Arc::new(config),
            #[cfg(feature = "db")]
            db: None,
        }
    }

    /// Context for the `cron` command's jobs: the command's config and pool.
    pub(super) fn from_command(ctx: &Context<'_>) -> Self {
        let job = Self::new("", ctx.config().clone());
        #[cfg(feature = "db")]
        let job = match ctx.db() {
            Ok(db) => job.with_db(db.clone()),
            Err(_) => job,
        };
        job
    }

    /// Same config and pool, for the named job.
    pub(super) fn for_job(&self, name: &'static str) -> Self {
        Self {
            name,
            ..self.clone()
        }
    }

    /// The running job's name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The application config, e.g. `ctx.config().section::<MyConfig>()`.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Attach the database pool (for tests; the `cron` command does it).
    #[cfg(feature = "db")]
    pub fn with_db(mut self, db: Db) -> Self {
        self.db = Some(db);
        self
    }

    /// The database pool, e.g. `.fetch_all(ctx.db()?.pool())`.
    #[cfg(feature = "db")]
    pub fn db(&self) -> Result<&Db> {
        self.db
            .as_ref()
            .ok_or_else(|| Error::msg("no database in this job context"))
    }
}
