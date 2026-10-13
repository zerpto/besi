use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use croner::Cron;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use super::plan::Entry;
use super::{Job, JobContext};
use crate::{Error, Result};

/// Run every active entry on its schedule until `shutdown` resolves, then
/// wait up to `shutdown_timeout` for runs in progress.
pub(super) async fn run(
    entries: Vec<Entry>,
    base: JobContext,
    shutdown_timeout: Duration,
    shutdown: impl Future<Output = ()>,
) -> Result<()> {
    for entry in entries.iter().filter(|e| !e.is_active()) {
        let reason = if entry.enabled {
            "no schedule"
        } else {
            "disabled"
        };
        tracing::info!(job = entry.job.name(), "not scheduled: {reason}");
    }
    let active: Vec<Entry> = entries.into_iter().filter(Entry::is_active).collect();
    if active.is_empty() {
        return Err(Error::msg(
            "no cron jobs are scheduled; set cron.jobs.<name>.schedule in config",
        ));
    }

    let stop = CancellationToken::new();
    let loops = TaskTracker::new();
    let runs = TaskTracker::new();
    for entry in active {
        let (Some(expr), Some(schedule)) = (entry.expr, entry.schedule) else {
            continue;
        };
        tracing::info!(job = entry.job.name(), "scheduled: {expr} (UTC)");
        loops.spawn(schedule_loop(
            entry.job,
            schedule,
            base.clone(),
            stop.clone(),
            runs.clone(),
        ));
    }
    tracing::info!("scheduler started");

    shutdown.await;
    stop.cancel();
    loops.close();
    loops.wait().await;

    runs.close();
    if tokio::time::timeout(shutdown_timeout, runs.wait())
        .await
        .is_err()
    {
        tracing::warn!(
            "{} job run(s) still in progress after {} s; exiting anyway",
            runs.len(),
            shutdown_timeout.as_secs_f32()
        );
    }
    tracing::info!("scheduler stopped");
    Ok(())
}

/// Sleep until the job's next time, start a run, repeat. A run still in
/// progress at the next time makes that time be skipped.
async fn schedule_loop(
    job: Job,
    schedule: Cron,
    base: JobContext,
    stop: CancellationToken,
    runs: TaskTracker,
) {
    let running = Arc::new(AtomicBool::new(false));
    let mut last: Option<DateTime<Utc>> = None;

    loop {
        let now = Utc::now();
        // Never pick the same time twice, even if the wall clock lags the timer.
        let from = last.map_or(now, |last| last.max(now));
        let next = match schedule.find_next_occurrence(&from, false) {
            Ok(next) => next,
            Err(err) => {
                tracing::error!(job = job.name(), "no next run time ({err}); job stopped");
                return;
            }
        };
        let wait = (next - now).to_std().unwrap_or(Duration::ZERO);

        tokio::select! {
            () = tokio::time::sleep(wait) => {}
            () = stop.cancelled() => return,
        }
        last = Some(next);

        if running.swap(true, Ordering::SeqCst) {
            tracing::warn!(
                job = job.name(),
                "previous run still in progress; skipping this run"
            );
            continue;
        }
        let (job, ctx, running) = (job.clone(), base.for_job(job.name()), running.clone());
        runs.spawn(async move {
            let _ = execute(&job, ctx).await;
            running.store(false, Ordering::SeqCst);
        });
    }
}

/// Run a job once, logging start, outcome and duration. A panic inside the
/// job is caught and reported as an error.
pub(super) async fn execute(job: &Job, ctx: JobContext) -> Result<()> {
    let name = job.name();
    tracing::info!(job = name, "started");
    let started = Instant::now();

    // Own task, so a panic surfaces as a JoinError here.
    let result = match tokio::spawn(job.call(ctx)).await {
        Ok(result) => result,
        Err(err) => Err(Error::msg(format!("job panicked: {err}"))),
    };

    let elapsed_ms = started.elapsed().as_millis();
    match &result {
        Ok(()) => tracing::info!(job = name, elapsed_ms, "finished"),
        Err(err) => tracing::error!(job = name, elapsed_ms, "failed: {err}"),
    }
    result
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;
    use crate::Config;
    use crate::cron::{CronConfig, JobSettings, plan::plan};

    const EVERY_SECOND: &str = "* * * * * *";

    fn entries(job: Job, schedule: Option<&str>) -> Vec<Entry> {
        let mut config = CronConfig::default();
        config.jobs.insert(
            job.name().to_string(),
            JobSettings {
                schedule: schedule.map(String::from),
                enabled: true,
            },
        );
        plan(vec![job], &config).unwrap()
    }

    fn base() -> JobContext {
        JobContext::new("", Config::default())
    }

    async fn after(ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }

    #[tokio::test]
    async fn errors_when_nothing_is_scheduled() {
        let job = Job::new("idle", |_| async { Ok(()) });
        let err = run(entries(job, None), base(), Duration::ZERO, after(0))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("no cron jobs are scheduled"),
            "{err}"
        );
    }

    /// Job takes 1.5 s on an every-second schedule, for 3.5 s: whatever the
    /// start offset, that's run, skip, run (and maybe a final skip). The last
    /// run is still going at shutdown, and shutdown waits for it.
    #[tokio::test]
    async fn skips_overlapping_runs_and_waits_on_shutdown() {
        let started = Arc::new(AtomicUsize::new(0));
        let finished = Arc::new(AtomicUsize::new(0));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));

        let job = {
            let (started, finished, in_flight, max) = (
                started.clone(),
                finished.clone(),
                in_flight.clone(),
                max_in_flight.clone(),
            );
            Job::new("slow", move |_| {
                let (started, finished, in_flight, max) = (
                    started.clone(),
                    finished.clone(),
                    in_flight.clone(),
                    max.clone(),
                );
                async move {
                    started.fetch_add(1, Ordering::SeqCst);
                    let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    max.fetch_max(now, Ordering::SeqCst);
                    after(1500).await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    finished.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            })
        };

        run(
            entries(job, Some(EVERY_SECOND)),
            base(),
            Duration::from_secs(5),
            after(3500),
        )
        .await
        .unwrap();

        assert_eq!(max_in_flight.load(Ordering::SeqCst), 1, "runs overlapped");
        assert_eq!(started.load(Ordering::SeqCst), 2, "expected run, skip, run");
        assert_eq!(finished.load(Ordering::SeqCst), 2, "shutdown didn't wait");
    }

    #[tokio::test]
    async fn shutdown_gives_up_after_timeout() {
        let job = Job::new("stuck", |_| async {
            after(60_000).await;
            Ok(())
        });
        let clock = Instant::now();
        run(
            entries(job, Some(EVERY_SECOND)),
            base(),
            Duration::from_millis(200),
            after(1500),
        )
        .await
        .unwrap();
        assert!(
            clock.elapsed() < Duration::from_secs(3),
            "{:?}",
            clock.elapsed()
        );
    }

    #[tokio::test]
    async fn a_panicking_job_keeps_its_schedule() {
        let calls = Arc::new(AtomicUsize::new(0));
        let job = {
            let calls = calls.clone();
            Job::new("flaky", move |_| {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    panic!("boom");
                }
            })
        };
        run(
            entries(job, Some(EVERY_SECOND)),
            base(),
            Duration::from_secs(1),
            after(2500),
        )
        .await
        .unwrap();
        assert!(calls.load(Ordering::SeqCst) >= 2, "loop died after a panic");
    }

    #[tokio::test]
    async fn execute_reports_failure_and_panic() {
        let failing = Job::new("failing", |_| async { Err(Error::msg("nope")) });
        assert_eq!(
            execute(&failing, base()).await.unwrap_err().to_string(),
            "nope"
        );

        let panicking = Job::new("panicking", |_| async { panic!("boom") });
        let err = execute(&panicking, base()).await.unwrap_err();
        assert!(err.to_string().starts_with("job panicked"), "{err}");
    }
}
