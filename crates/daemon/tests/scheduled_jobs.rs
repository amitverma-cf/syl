use daemon::jobs::{add_scheduled_job, load_scheduled_jobs, remove_scheduled_job, ScheduledJob};

fn temp_jobs_file() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "syl-scheduled-jobs-test-{}-{}.json",
        std::process::id(),
        uuid::Uuid::new_v4()
    ))
}

fn sample_job(id: &str) -> ScheduledJob {
    ScheduledJob {
        id: id.to_string(),
        name: "morning summary".to_string(),
        cron_expr: "0 0 9 * * *".to_string(),
        conversation_id: "conv-1".to_string(),
        prompt: "Summarize yesterday's notes.".to_string(),
        model: None,
    }
}

#[test]
fn load_scheduled_jobs_is_empty_when_file_missing() {
    let path = temp_jobs_file();
    assert!(load_scheduled_jobs(&path).is_empty());
}

#[test]
fn add_then_load_round_trips_a_job_via_disk() {
    let path = temp_jobs_file();
    add_scheduled_job(&path, sample_job("job-1")).unwrap();

    let jobs = load_scheduled_jobs(&path);
    assert_eq!(jobs, vec![sample_job("job-1")]);

    std::fs::remove_file(&path).ok();
}

#[test]
fn add_scheduled_job_replaces_an_existing_job_with_the_same_id() {
    let path = temp_jobs_file();
    add_scheduled_job(&path, sample_job("job-1")).unwrap();

    let mut updated = sample_job("job-1");
    updated.cron_expr = "0 30 9 * * *".to_string();
    add_scheduled_job(&path, updated.clone()).unwrap();

    let jobs = load_scheduled_jobs(&path);
    assert_eq!(jobs, vec![updated]);

    std::fs::remove_file(&path).ok();
}

#[test]
fn remove_scheduled_job_deletes_only_the_matching_job() {
    let path = temp_jobs_file();
    add_scheduled_job(&path, sample_job("job-1")).unwrap();
    add_scheduled_job(&path, sample_job("job-2")).unwrap();

    remove_scheduled_job(&path, "job-1").unwrap();

    let jobs = load_scheduled_jobs(&path);
    assert_eq!(jobs, vec![sample_job("job-2")]);

    std::fs::remove_file(&path).ok();
}
