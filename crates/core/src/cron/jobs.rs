use super::Job;

/// What the application supplies to the `cron` command: its jobs.
///
/// Implement this on a marker type and name it in the command:
/// `CronCommand<AppJobs>`. Schedules are not set here; they come from
/// `cron.jobs.<name>.schedule` in config.
pub trait CronJobs: 'static {
    fn jobs() -> Vec<Job>;
}
