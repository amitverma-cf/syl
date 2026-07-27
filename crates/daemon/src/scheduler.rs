//! Cron/time-based scheduled jobs and parallel task orchestration.

/// A job that runs on a recurring schedule.
pub struct ScheduledJob {
    /// The job's unique name.
    pub name: String,
    /// The cron expression describing when this job runs.
    pub cron_expr: String,
}
