use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use core_types::app_config::app_config;
use core_types::workspace_paths;
use daemon::events::{DaemonEvent, EventBus};
use daemon::scheduler::CronScheduler;
use tauri::{AppHandle, Manager};

use crate::scheduled_jobs::{register_persisted_jobs, SchedulerState};

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

pub async fn spawn(app: AppHandle) {
    let event_bus = app.state::<DaemonState>().event_bus.clone();

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

    let scheduler_state = SchedulerState::new(Arc::new(scheduler));
    register_persisted_jobs(&app, &scheduler_state).await;

    if let Err(err) = scheduler_state.scheduler.start().await {
        tracing::error!(?err, "failed to start the daemon cron scheduler");
        return;
    }
    app.manage(scheduler_state);
}

async fn poll_registry(event_bus: Arc<EventBus>) {
    let result = tauri::async_runtime::spawn_blocking(|| {
        let (engines_json, models_json) =
            plugin_registry::fetch_remote_registry(&app_config().registry_poll_url)?;
        // Validate (host-allowlisted URLs, safe library_file paths) and atomically
        // persist only if the whole fetched pair passes — a poll that fails
        // validation, or a network/parse error, leaves the last-known-good registry
        // files exactly as they were, so a compromised or malformed response can
        // never partially or fully overwrite what's already trusted.
        plugin_registry::apply_remote_registry(
            &workspace_paths::registry_dir(),
            &engines_json,
            &models_json,
            &app_config().registry_allowed_hosts,
        )
    })
    .await;
    match result {
        Ok(Ok(())) => {
            tracing::info!("polled and applied the remote plugin registry");
            event_bus.publish(DaemonEvent::RegistryPolled { ok: true });
        }
        Ok(Err(err)) => {
            tracing::warn!(
                ?err,
                "failed to poll or validate the remote plugin registry"
            );
            event_bus.publish(DaemonEvent::RegistryPolled { ok: false });
        }
        Err(err) => {
            tracing::error!(?err, "registry poll task panicked");
        }
    }
}
