use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub use uuid::Uuid;

use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

pub struct CronScheduler {
    inner: JobScheduler,
}

impl CronScheduler {
    pub async fn new() -> Result<Self, JobSchedulerError> {
        Ok(Self {
            inner: JobScheduler::new().await?,
        })
    }

    /// Registers a cron job (standard 6-field cron expression) that runs `task` on each firing,
    /// returning the job's id so it can later be removed via [`Self::remove_job`]. The job keeps
    /// firing independent of the app's main window, driven by this scheduler's own tokio task
    /// started via `start`.
    pub async fn add_cron_job<F>(
        &self,
        cron_expr: &str,
        task: Arc<F>,
    ) -> Result<Uuid, JobSchedulerError>
    where
        F: Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
    {
        let job = Job::new_async(cron_expr, move |_uuid, _lock| {
            let task = task.clone();
            Box::pin(async move { task().await })
        })?;
        self.inner.add(job).await
    }

    /// Unregisters a previously-added job so it stops firing.
    pub async fn remove_job(&self, job_id: &Uuid) -> Result<(), JobSchedulerError> {
        self.inner.remove(job_id).await
    }

    pub async fn start(&self) -> Result<(), JobSchedulerError> {
        self.inner.start().await
    }
}
