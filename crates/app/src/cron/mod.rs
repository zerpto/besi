//! Scheduled jobs run by `app cron`: `jobs/` holds one module per job,
//! `registry.rs` lists them. Schedules live in config, not here.

mod jobs;
mod registry;

pub use registry::AppJobs;
