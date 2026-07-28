use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use daemon::events::{DaemonEvent, EventBus};
use daemon::scheduler::CronScheduler;

const REGISTRY_BASE_URL: &str = "https://raw.githubusercontent.com/amitverma-cf/syl/main/registry";

pub struct DaemonState {
    pub event_bus: Arc<EventBus>,
}

impl Default for DaemonState {
    fn default() -> Self {
        Self {
            event_bus: Arc::new(EventBus::new(64)),
        }
    }
}

/// Starts the background scheduler and registers the registry-poll job. Runs independent of
/// whether the main window is open, so it must be spawned once at app startup, not lazily on
/// first UI interaction.
pub async fn spawn(event_bus: Arc<EventBus>) {
    let scheduler = match CronScheduler::new().await {
        Ok(scheduler) => scheduler,
        Err(err) => {
            tracing::error!(?err, "failed to create the daemon cron scheduler");
            return;
        }
    };

    let job_event_bus = event_bus.clone();
    let job_result = scheduler
        .add_cron_job(
            "0 0 */6 * * *",
            Arc::new(move || {
                let event_bus = job_event_bus.clone();
                Box::pin(poll_registry(event_bus)) as Pin<Box<dyn Future<Output = ()> + Send>>
            }),
        )
        .await;
    if let Err(err) = job_result {
        tracing::error!(?err, "failed to register the registry-poll cron job");
        return;
    }

    if let Err(err) = scheduler.start().await {
        tracing::error!(?err, "failed to start the daemon cron scheduler");
        return;
    }

    // The scheduler drives itself via its own tokio task once started; keep it alive for the
    // life of the process rather than dropping it at the end of this async fn.
    std::mem::forget(scheduler);
}

async fn poll_registry(event_bus: Arc<EventBus>) {
    let result = tauri::async_runtime::spawn_blocking(|| {
        plugin_registry::fetch_remote_registry(REGISTRY_BASE_URL)
    })
    .await;
    match result {
        Ok(Ok(_)) => {
            tracing::info!("polled the remote plugin registry");
            event_bus.publish(DaemonEvent::RegistryPolled { ok: true });
        }
        Ok(Err(err)) => {
            tracing::warn!(?err, "failed to poll the remote plugin registry");
            event_bus.publish(DaemonEvent::RegistryPolled { ok: false });
        }
        Err(err) => {
            tracing::error!(?err, "registry poll task panicked");
        }
    }
}
