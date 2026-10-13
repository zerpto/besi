# Cron (scheduled jobs)

`besi::CronCommand<J>` (feature `cron`) runs the app's jobs on cron
schedules. The scheduler lives in `besi`; the app only lists its jobs. **When**
a job runs comes only from config.

| Part | Owned by | Code |
|---|---|---|
| Scheduler, validation, overlap skip, logging, graceful shutdown, `cron` config | `besi` | [`src/cron/`](../src/cron/) |
| Job list (`AppJobs: CronJobs`) | your app | `cron/registry.rs` ([recommended layout](getting-started.md#recommended-project-layout)) |
| Job code | your app | `cron/jobs/`, one file per job |

In examples below the app registers the command as `cron`, has a `db_ping`
job, and `app` stands for its binary.

Built on [croner](https://crates.io/crates/croner) (cron expressions) and
[tokio-util](https://crates.io/crates/tokio-util) (cancellation and task
tracking for shutdown).

## Commands

```sh
app cron                 # run the scheduler until Ctrl+C / SIGTERM
app cron --list          # jobs, schedules, status, next run time; then exit
app cron run db_ping     # run one job now, once, then exit
```

`--list` output:

```
JOB      SCHEDULE        STATUS  NEXT RUN (UTC)
db_ping  */30 * * * * *  active  2026-10-12 09:55:30
```

Status is `active` (enabled and scheduled), `disabled`, or `no schedule`.

`cron run <job>` ignores the schedule and `enabled`, so it works for any
registered job. It prints `<job> finished in N ms`, or exits 1 with the job's
error.

## Configuration

Section `cron` (see [config.md](config.md) for layering):

```yaml
cron:
  shutdown_timeout_secs: 30
  jobs:
    db_ping:
      schedule: "*/30 * * * * *"
      enabled: true
```

| Key | Env | Default |
|---|---|---|
| `cron.jobs.<name>.schedule` | `CRON__JOBS__<NAME>__SCHEDULE` | none (job doesn't run) |
| `cron.jobs.<name>.enabled` | `CRON__JOBS__<NAME>__ENABLED` | `true` |
| `cron.shutdown_timeout_secs` | `CRON__SHUTDOWN_TIMEOUT_SECS` | `30` |

Example: `CRON__JOBS__DB_PING__SCHEDULE="0 */5 * * * *"`.

### Schedule format

Standard cron, **always UTC**:

| Fields | Format | Example | Meaning |
|---|---|---|---|
| 5 | `min hour day month weekday` | `*/5 * * * *` | every 5 minutes |
| 6 | `sec min hour day month weekday` | `*/30 * * * * *` | every 30 seconds |

Also accepted: `@hourly`, `@daily`, `@weekly`, `@monthly`, `@yearly`, ranges
(`1-5`), lists (`1,15`), steps (`*/10`), and names (`MON`, `JAN`). Quote
expressions in YAML.

### Validation (at startup, before anything runs)

All of these stop the command (including `--list`) with exit 1:

| Problem | Error |
|---|---|
| Config names a job that isn't registered (typo) | `cron.jobs.db_pign: no job with that name is registered` |
| Expression doesn't parse | `cron.jobs.db_ping.schedule: invalid cron expression "...": ...` |
| Job name isn't lowercase `snake_case` | `cron job name "DbPing" must be lowercase snake_case (a-z, 0-9, _)` |
| Same name registered twice | `cron job "db_ping" is registered twice` |
| `app cron` with no active job | `no cron jobs are scheduled; set cron.jobs.<name>.schedule in config` |

Job names must be lowercase because env keys are lowercased: a job named
`DbPing` could never be scheduled from the environment.

## Runtime behavior

- **One loop per active job.** Each sleeps until its next time, starts a run,
  and repeats. Jobs don't block each other.
- **Overlap: skip.** If a job's previous run is still going when its next time
  comes, that time is skipped and logged:
  `WARN ... previous run still in progress; skipping this run job="db_ping"`.
  At most one run per job at a time.
- **Failures don't stop the schedule.** A job returning `Err` is logged as
  `ERROR ... failed: <error> job="..." elapsed_ms=...`. A job that panics is
  caught and logged the same way (`job panicked: ...`). The next run happens
  on schedule.
- **Logging** goes to stderr (`RUST_LOG`, default `info`): one line when a run
  starts and one when it finishes or fails, with duration.
- **Shutdown.** On Ctrl+C / SIGTERM, no new runs start, and the scheduler waits
  up to `shutdown_timeout_secs` for runs in progress. After that it logs how
  many were still running and exits anyway. Exit code is 0 either way.
- **Missed times aren't caught up.** If the process is down at 03:00, the
  03:00 run doesn't happen later. Schedules only look forward.
- **Single process.** Running `app cron` in two places runs every job twice.
  There is no distributed lock (yet); run one scheduler per environment.

## Adding a job

1. Create `cron/jobs/<name>.rs`:

   ```rust
   use besi::{JobContext, Result};

   pub async fn run(ctx: JobContext) -> Result<()> {
       let rows = besi::sqlx::query("DELETE FROM sessions WHERE expires_at < now()")
           .execute(ctx.db()?.pool())
           .await?;
       // ctx.config().section::<MyConfig>()? for settings
       Ok(())
   }
   ```

2. Declare it in `cron/jobs/mod.rs`: `pub mod <name>;`.

3. Register it in the job list:

   ```rust
   use besi::{CronJobs, Job};

   /// Marker type naming this app's jobs: `besi::CronCommand<AppJobs>`.
   #[derive(Debug)]
   pub struct AppJobs;

   impl CronJobs for AppJobs {
       fn jobs() -> Vec<Job> {
           vec![
               Job::new("db_ping", jobs::db_ping::run),
               Job::new("cleanup_sessions", jobs::cleanup_sessions::run),
           ]
       }
   }
   ```

   The first time, also add the command to the app's `Commands` enum:
   `Cron(besi::CronCommand<AppJobs>)`, dispatched with `cmd.run(ctx).await`.

4. Schedule it in config (`cron.jobs.cleanup_sessions.schedule`). Check with
   `app cron --list`; try it with `app cron run cleanup_sessions`.

A job gets a `JobContext`: `name()`, `config()`, and `db()` (feature `db`), the
same process-wide pool commands and handlers use. Jobs have no stdout; report
problems by returning `Err`.

Make jobs safe to run late or be skipped (overlap, restarts), and ideally
idempotent.

## Testing a job

Jobs are plain async functions. Build a `JobContext` and call them:

```rust
#[tokio::test]
async fn fails_when_database_unreachable() {
    let db = Db::connect_lazy(&DatabaseConfig { host: "127.0.0.1".into(), port: 1, ..Default::default() })?;
    let ctx = JobContext::new("db_ping", Config::default()).with_db(db);
    assert!(run(ctx).await.is_err());
}
```

For reference, a `db_ping` job is just:

```rust
pub async fn run(ctx: JobContext) -> Result<()> {
    ctx.db()?.ping().await
}
```

## Internals

| File | Contains |
|---|---|
| `jobs.rs` | `CronJobs` trait: what the app supplies (`fn jobs() -> Vec<Job>`) |
| `job.rs` | `Job`: a name plus an async function |
| `context.rs` | `JobContext`: name, config, db |
| `settings.rs` | `CronConfig` / `JobSettings` (`cron` section) |
| `plan.rs` | matches jobs to config and validates everything |
| `scheduler.rs` | per-job loops, overlap skip, `execute` (logging, panic capture), shutdown wait |
| `command.rs` | `CronCommand<J>`: `--list`, `run <job>`, or the scheduler |

Shared with `api` in [`src/daemon/`](../src/daemon/):
stderr logging setup and the Ctrl+C / SIGTERM signal.

Design notes:

- **Same contract as every command.** `CronCommand<J: CronJobs>` implements
  `Command`; the app registers `Cron(besi::CronCommand<AppJobs>)` and
  dispatches with `cmd.run(ctx).await`, the same pattern as `ApiCommand<R>`.
- **Why croner and not a full scheduler crate.** Full schedulers don't skip
  overlapping runs or wait for running jobs on shutdown out of the box; the
  loop that adds those is about 60 lines.
- **Clock.** Next times come from the wall clock (`chrono::Utc::now`), and
  sleeping uses tokio's timer. A time is never picked twice, even if the wall
  clock lags the timer.
- The scheduler tests use real time (about 4 s total). tokio's paused clock
  can't be used, because the wall clock doesn't pause with it.
