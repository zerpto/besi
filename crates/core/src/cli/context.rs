use std::io::Write;

use crate::Config;
#[cfg(feature = "db")]
use crate::{Db, Error, Result};

/// Everything a command needs from the outside world.
///
/// Commands write through the context instead of printing directly, so
/// tests can capture output by passing a buffer.
pub struct Context<'a> {
    out: &'a mut dyn Write,
    config: &'a Config,
    #[cfg(feature = "db")]
    db: Option<Db>,
}

impl<'a> Context<'a> {
    pub fn new(out: &'a mut dyn Write, config: &'a Config) -> Self {
        Self {
            out,
            config,
            #[cfg(feature = "db")]
            db: None,
        }
    }

    /// Standard output (or the test buffer).
    pub fn out(&mut self) -> &mut dyn Write {
        self.out
    }

    /// Merged configuration (YAML < .env < environment).
    pub fn config(&self) -> &Config {
        self.config
    }

    /// Attach the database pool. The runner does this for every command.
    #[cfg(feature = "db")]
    pub fn with_db(mut self, db: Db) -> Self {
        self.db = Some(db);
        self
    }

    /// The database pool. Always set when run by `besi::run`; errors only
    /// in a test `Context` built without [`Context::with_db`].
    #[cfg(feature = "db")]
    pub fn db(&self) -> Result<&Db> {
        self.db
            .as_ref()
            .ok_or_else(|| Error::msg("no database in this context"))
    }
}
