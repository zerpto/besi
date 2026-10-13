//! The app's job list. To add a job: create `jobs/<name>.rs` with
//! `pub async fn run(ctx: JobContext) -> Result<()>`, declare it in
//! `jobs/mod.rs`, and add a `Job::new` line here. Then give it a schedule in
//! config: `cron.jobs.<name>.schedule`.

use besi::{CronJobs, Job};

use super::jobs;

/// Marker type naming this app's jobs: `besi::CronCommand<AppJobs>`.
#[derive(Debug)]
pub struct AppJobs;

impl CronJobs for AppJobs {
    fn jobs() -> Vec<Job> {
        vec![Job::new("db_ping", jobs::db_ping::run)]
    }
}
