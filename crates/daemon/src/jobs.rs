use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum JobsError {
    #[error("failed to read or write scheduled jobs file: {0}")]
    Storage(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A user-defined recurring chat turn: firing `cron_expr` sends `prompt` (using cloud `model`
/// if set, otherwise the local engine) into `conversation_id`, the same way a real chat message
/// would — an unattended turn through the same tool-calling loop and flow FSM as a live chat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledJob {
    pub id: String,
    pub name: String,
    pub cron_expr: String,
    pub conversation_id: String,
    pub prompt: String,
    pub model: Option<String>,
}

pub fn load_scheduled_jobs(path: &Path) -> Vec<ScheduledJob> {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_scheduled_jobs(path: &Path, jobs: &[ScheduledJob]) -> Result<(), JobsError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(jobs)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn add_scheduled_job(path: &Path, job: ScheduledJob) -> Result<ScheduledJob, JobsError> {
    let mut jobs = load_scheduled_jobs(path);
    jobs.retain(|existing| existing.id != job.id);
    jobs.push(job.clone());
    save_scheduled_jobs(path, &jobs)?;
    Ok(job)
}

pub fn remove_scheduled_job(path: &Path, id: &str) -> Result<(), JobsError> {
    let mut jobs = load_scheduled_jobs(path);
    jobs.retain(|job| job.id != id);
    save_scheduled_jobs(path, &jobs)
}
