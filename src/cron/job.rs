use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::JobContext;
use crate::Result;

type BoxFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
type JobFn = dyn Fn(JobContext) -> BoxFuture + Send + Sync;

/// A named unit of scheduled work.
///
/// ```ignore
/// Job::new("db_ping", jobs::db_ping::run)   // pub async fn run(ctx: JobContext) -> Result<()>
/// ```
///
/// The name is the config key (`cron.jobs.db_ping.schedule`), so it must be
/// lowercase `snake_case`: env keys are lowercased, so anything else could
/// never be configured from the environment.
#[derive(Clone)]
pub struct Job {
    name: &'static str,
    run: Arc<JobFn>,
}

impl Job {
    pub fn new<F, Fut>(name: &'static str, run: F) -> Self
    where
        F: Fn(JobContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        Self {
            name,
            run: Arc::new(move |ctx| Box::pin(run(ctx))),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub(super) fn call(&self, ctx: JobContext) -> BoxFuture {
        (self.run)(ctx)
    }
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job").field("name", &self.name).finish()
    }
}
