use std::sync::Arc;

use crate::Config;
#[cfg(feature = "db")]
use crate::{Db, Error, Result};

/// Shared state available to every handler via `State<ApiState>`.
#[derive(Debug, Clone)]
pub struct ApiState {
    config: Arc<Config>,
    #[cfg(feature = "db")]
    db: Option<Db>,
}

impl ApiState {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            #[cfg(feature = "db")]
            db: None,
        }
    }

    /// The application config, e.g. `state.config().section::<MyConfig>()`.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Attach the database pool. `app api` does this from the command's
    /// context, so handlers share the process-wide pool.
    #[cfg(feature = "db")]
    pub fn with_db(mut self, db: Db) -> Self {
        self.db = Some(db);
        self
    }

    /// The database pool, e.g. `.fetch_all(state.db()?.pool())`. Errors only
    /// in a test state built without [`ApiState::with_db`].
    #[cfg(feature = "db")]
    pub fn db(&self) -> Result<&Db> {
        self.db
            .as_ref()
            .ok_or_else(|| Error::msg("no database in this state"))
    }
}
