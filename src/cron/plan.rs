//! Match the app's jobs against `cron` config, and reject anything that
//! doesn't line up, before a single job runs.

use std::collections::HashSet;
use std::str::FromStr;

use croner::Cron;

use super::{CronConfig, Job};
use crate::{Error, Result};

/// One registered job and how config says to run it.
#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub job: Job,
    /// The expression as written in config, for display.
    pub expr: Option<String>,
    pub schedule: Option<Cron>,
    pub enabled: bool,
}

impl Entry {
    /// Runs on its own: enabled and has a schedule.
    pub fn is_active(&self) -> bool {
        self.enabled && self.schedule.is_some()
    }
}

/// Validate `jobs` and `config` together:
///
/// - job names are lowercase `snake_case` and unique;
/// - every `cron.jobs.<name>` refers to a registered job (catches typos);
/// - every schedule parses.
pub(super) fn plan(jobs: Vec<Job>, config: &CronConfig) -> Result<Vec<Entry>> {
    let mut names = HashSet::new();
    for job in &jobs {
        let name = job.name();
        if !is_valid_name(name) {
            return Err(Error::msg(format!(
                "cron job name {name:?} must be lowercase snake_case (a-z, 0-9, _)"
            )));
        }
        if !names.insert(name) {
            return Err(Error::msg(format!("cron job {name:?} is registered twice")));
        }
    }

    let mut unknown: Vec<&String> = config
        .jobs
        .keys()
        .filter(|key| !names.contains(key.as_str()))
        .collect();
    unknown.sort();
    if let Some(key) = unknown.first() {
        return Err(Error::msg(format!(
            "cron.jobs.{key}: no job with that name is registered"
        )));
    }

    jobs.into_iter()
        .map(|job| {
            let settings = config.jobs.get(job.name()).cloned().unwrap_or_default();
            let schedule = settings
                .schedule
                .as_deref()
                .map(|expr| {
                    Cron::from_str(expr).map_err(|err| {
                        Error::msg(format!(
                            "cron.jobs.{}.schedule: invalid cron expression {expr:?}: {err}",
                            job.name()
                        ))
                    })
                })
                .transpose()?;
            Ok(Entry {
                job,
                expr: settings.schedule,
                schedule,
                enabled: settings.enabled,
            })
        })
        .collect()
}

fn is_valid_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cron::JobSettings;

    fn job(name: &'static str) -> Job {
        Job::new(name, |_| async { Ok(()) })
    }

    fn config(jobs: &[(&str, Option<&str>, bool)]) -> CronConfig {
        CronConfig {
            jobs: jobs
                .iter()
                .map(|(name, schedule, enabled)| {
                    (
                        name.to_string(),
                        JobSettings {
                            schedule: schedule.map(String::from),
                            enabled: *enabled,
                        },
                    )
                })
                .collect(),
            ..CronConfig::default()
        }
    }

    fn err(jobs: Vec<Job>, config: &CronConfig) -> String {
        plan(jobs, config).unwrap_err().to_string()
    }

    #[test]
    fn schedules_come_from_config() {
        let entries = plan(
            vec![job("a"), job("b"), job("c")],
            &config(&[
                ("a", Some("*/5 * * * *"), true),
                ("b", Some("0 * * * *"), false),
            ]),
        )
        .unwrap();
        let active: Vec<_> = entries
            .iter()
            .filter(|e| e.is_active())
            .map(|e| e.job.name())
            .collect();
        // b is disabled; c has no schedule.
        assert_eq!(active, ["a"]);
    }

    #[test]
    fn accepts_seconds_field() {
        plan(
            vec![job("a")],
            &config(&[("a", Some("*/10 * * * * *"), true)]),
        )
        .unwrap();
    }

    #[test]
    fn rejects_invalid_schedule() {
        let msg = err(
            vec![job("a")],
            &config(&[("a", Some("every minute"), true)]),
        );
        assert!(
            msg.starts_with("cron.jobs.a.schedule: invalid cron expression"),
            "{msg}"
        );
    }

    #[test]
    fn rejects_config_for_unknown_job() {
        let msg = err(
            vec![job("cleanup")],
            &config(&[("clenaup", Some("* * * * *"), true)]),
        );
        assert_eq!(
            msg,
            "cron.jobs.clenaup: no job with that name is registered"
        );
    }

    #[test]
    fn rejects_bad_and_duplicate_names() {
        let none = CronConfig::default();
        assert!(err(vec![job("DbPing")], &none).contains("lowercase snake_case"));
        assert!(err(vec![job("db-ping")], &none).contains("lowercase snake_case"));
        assert!(err(vec![job("a"), job("a")], &none).contains("registered twice"));
    }
}
