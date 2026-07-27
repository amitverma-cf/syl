//! Cron/time-based scheduled jobs and parallel task orchestration.
//! Real implementation will use tokio-cron-scheduler + tokio::task::JoinSet.

pub struct ScheduledJob {
    pub name: String,
    pub cron_expr: String,
}
