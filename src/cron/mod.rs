//! Job scheduler and the `cron` command (feature `cron`).
//!
//! The app lists its jobs by implementing [`CronJobs`]; *when* each one runs
//! comes only from config (`cron.jobs.<name>.schedule`). The core validates
//! the list against config, runs each scheduled job on its own loop, skips a
//! run if the previous one is still going, and on shutdown waits (bounded)
//! for running jobs to finish.

mod command;
mod context;
mod job;
mod jobs;
mod plan;
mod scheduler;
mod settings;

pub use command::CronCommand;
pub use context::JobContext;
pub use job::Job;
pub use jobs::CronJobs;
pub use settings::{CronConfig, JobSettings};
