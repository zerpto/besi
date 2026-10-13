use std::marker::PhantomData;
use std::time::Instant;

use chrono::Utc;
use clap::{Args, Subcommand};

use super::plan::{Entry, plan};
use super::{CronConfig, CronJobs, JobContext, scheduler};
use crate::{Command, Context, Error, Result};

/// The `cron` subcommand, on `J`'s jobs.
///
/// - `cron`: run the scheduler until Ctrl+C or SIGTERM
/// - `cron --list`: show jobs, schedules and next run times
/// - `cron run <job>`: run one job now and exit
///
/// Like every other command, dispatched with plain [`Command::run`]; the
/// app's jobs arrive through the type parameter.
#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct CronCommand<J: CronJobs> {
    /// List registered jobs with their schedule and next run, then exit
    #[arg(long)]
    list: bool,

    #[command(subcommand)]
    action: Option<CronAction>,

    #[arg(skip)]
    jobs: PhantomData<fn() -> J>,
}

#[derive(Debug, Subcommand)]
enum CronAction {
    /// Run one job now, once, then exit (ignores its schedule and `enabled`)
    Run {
        /// Job name, as shown by `cron --list`
        job: String,
    },
}

impl<J: CronJobs> Command for CronCommand<J> {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        let config: CronConfig = ctx.config().section()?;
        let entries = plan(J::jobs(), &config)?;

        if self.list {
            return list(ctx, &entries);
        }

        let base = JobContext::from_command(ctx);
        crate::daemon::init_logging();
        match &self.action {
            Some(CronAction::Run { job }) => run_now(ctx, &entries, &base, job).await,
            None => {
                scheduler::run(
                    entries,
                    base,
                    config.shutdown_timeout(),
                    crate::daemon::shutdown_signal(),
                )
                .await
            }
        }
    }
}

/// Table of jobs: name, schedule, status, next run (UTC).
fn list(ctx: &mut Context<'_>, entries: &[Entry]) -> Result<()> {
    let now = Utc::now();
    let mut rows = vec![[
        "JOB".to_string(),
        "SCHEDULE".to_string(),
        "STATUS".to_string(),
        "NEXT RUN (UTC)".to_string(),
    ]];
    for entry in entries {
        let status = match (entry.enabled, &entry.schedule) {
            (_, None) => "no schedule",
            (false, Some(_)) => "disabled",
            (true, Some(_)) => "active",
        };
        let next = match &entry.schedule {
            Some(schedule) if entry.enabled => schedule
                .find_next_occurrence(&now, false)
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|_| "never".into()),
            _ => "-".into(),
        };
        rows.push([
            entry.job.name().to_string(),
            entry.expr.clone().unwrap_or_else(|| "-".into()),
            status.to_string(),
            next,
        ]);
    }

    let widths: Vec<usize> = (0..4)
        .map(|col| rows.iter().map(|row| row[col].len()).max().unwrap_or(0))
        .collect();
    for row in &rows {
        let line = format!(
            "{:w0$}  {:w1$}  {:w2$}  {}",
            row[0],
            row[1],
            row[2],
            row[3],
            w0 = widths[0],
            w1 = widths[1],
            w2 = widths[2],
        );
        writeln!(ctx.out(), "{}", line.trim_end())?;
    }
    Ok(())
}

async fn run_now(
    ctx: &mut Context<'_>,
    entries: &[Entry],
    base: &JobContext,
    name: &str,
) -> Result<()> {
    let entry = entries
        .iter()
        .find(|entry| entry.job.name() == name)
        .ok_or_else(|| Error::msg(format!("no cron job named {name:?}; see `cron --list`")))?;

    let started = Instant::now();
    scheduler::execute(&entry.job, base.for_job(entry.job.name())).await?;
    writeln!(
        ctx.out(),
        "{name} finished in {} ms",
        started.elapsed().as_millis()
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use crate::cron::{Job, JobSettings};

    #[derive(Debug)]
    struct TestJobs;
    impl CronJobs for TestJobs {
        fn jobs() -> Vec<Job> {
            vec![
                Job::new("ok_job", |_| async { Ok(()) }),
                Job::new("bad_job", |_| async { Err(Error::msg("it broke")) }),
                Job::new("idle_job", |_| async { Ok(()) }),
            ]
        }
    }

    fn entries() -> Vec<Entry> {
        let mut config = CronConfig::default();
        let set = |schedule: &str, enabled| JobSettings {
            schedule: Some(schedule.into()),
            enabled,
        };
        config
            .jobs
            .insert("ok_job".into(), set("*/5 * * * *", true));
        config
            .jobs
            .insert("bad_job".into(), set("0 3 * * *", false));
        plan(TestJobs::jobs(), &config).unwrap()
    }

    fn base() -> JobContext {
        JobContext::new("", Config::default())
    }

    fn output(buf: Vec<u8>) -> String {
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn list_shows_every_job() {
        let (mut buf, config) = (Vec::new(), Config::default());
        list(&mut Context::new(&mut buf, &config), &entries()).unwrap();
        let out = output(buf);
        let lines: Vec<&str> = out.lines().collect();

        assert!(lines[0].starts_with("JOB"), "{out}");
        assert!(lines[1].starts_with("ok_job"), "{out}");
        assert!(
            lines[1].contains("*/5 * * * *") && lines[1].contains("active"),
            "{out}"
        );
        assert!(
            lines[2].contains("disabled") && lines[2].ends_with('-'),
            "{out}"
        );
        assert!(lines[3].contains("no schedule"), "{out}");
    }

    #[tokio::test]
    async fn run_now_runs_any_job_by_name() {
        let (mut buf, config) = (Vec::new(), Config::default());
        let mut ctx = Context::new(&mut buf, &config);
        // idle_job has no schedule; run still works.
        run_now(&mut ctx, &entries(), &base(), "idle_job")
            .await
            .unwrap();
        assert!(output(buf).starts_with("idle_job finished in"));
    }

    #[tokio::test]
    async fn run_now_propagates_failure() {
        let (mut buf, config) = (Vec::new(), Config::default());
        let mut ctx = Context::new(&mut buf, &config);
        let err = run_now(&mut ctx, &entries(), &base(), "bad_job")
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "it broke");
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn run_now_rejects_unknown_job() {
        let (mut buf, config) = (Vec::new(), Config::default());
        let mut ctx = Context::new(&mut buf, &config);
        let err = run_now(&mut ctx, &entries(), &base(), "nope")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no cron job named \"nope\""));
    }
}
