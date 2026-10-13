use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

use crate::ConfigSection;

/// `cron:` in config.yaml.
///
/// ```yaml
/// cron:
///   shutdown_timeout_secs: 30
///   jobs:
///     db_ping:
///       schedule: "*/30 * * * * *"   # sec min hour day month weekday
///       enabled: true
/// ```
///
/// Env: `CRON__JOBS__DB_PING__SCHEDULE`, `CRON__JOBS__DB_PING__ENABLED`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CronConfig {
    /// Per-job settings, keyed by job name.
    pub jobs: HashMap<String, JobSettings>,
    /// On shutdown, how long to wait for running jobs before exiting anyway.
    pub shutdown_timeout_secs: u64,
}

impl Default for CronConfig {
    fn default() -> Self {
        Self {
            jobs: HashMap::new(),
            shutdown_timeout_secs: 30,
        }
    }
}

impl ConfigSection for CronConfig {
    const KEY: &'static str = "cron";
}

impl CronConfig {
    pub(super) fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.shutdown_timeout_secs)
    }
}

/// `cron.jobs.<name>`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct JobSettings {
    /// Cron expression, 5 fields (`min hour day month weekday`) or 6 with
    /// seconds first. Times are UTC. No schedule means the job never runs on
    /// its own (it can still be run with `cron run <name>`).
    pub schedule: Option<String>,
    /// `false` keeps the schedule but stops the job from running.
    pub enabled: bool,
}

impl Default for JobSettings {
    fn default() -> Self {
        Self {
            schedule: None,
            enabled: true,
        }
    }
}
