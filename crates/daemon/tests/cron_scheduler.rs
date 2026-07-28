use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use daemon::scheduler::CronScheduler;

#[tokio::test]
async fn a_cron_job_fires_and_runs_the_task() {
    let scheduler = CronScheduler::new().await.unwrap();
    let run_count = Arc::new(AtomicU32::new(0));

    let counter = run_count.clone();
    scheduler
        .add_cron_job(
            "1/1 * * * * *",
            Arc::new(move || {
                let counter = counter.clone();
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                }) as Pin<Box<dyn Future<Output = ()> + Send>>
            }),
        )
        .await
        .unwrap();

    scheduler.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2500)).await;

    assert!(
        run_count.load(Ordering::SeqCst) >= 1,
        "expected the every-second cron job to have fired at least once"
    );
}
