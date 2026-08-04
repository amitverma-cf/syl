use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use core_types::app_config::app_config;
use core_types::workspace_paths;
use daemon::events::{DaemonEvent, EventBus};
use daemon::scheduler::CronScheduler;
use tauri::{AppHandle, Emitter, Manager};

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

    spawn_frontend_bridge(app.clone(), event_bus.clone());

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

/// Forwards `DaemonEvent`s the frontend actually needs to know about as real
/// Tauri events — today just `LocalModelCrashed`, mirroring the existing
/// `"tool-permission-timeout"` listener pattern (`permission.rs`) so a
/// crashed local model surfaces as a toast instead of only a backend log
/// line.
fn spawn_frontend_bridge(app: AppHandle, event_bus: Arc<EventBus>) {
    let mut receiver = event_bus.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(DaemonEvent::LocalModelCrashed { name }) => {
                    let _ = app.emit("local-model-crashed", name);
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn poll_registry(event_bus: Arc<EventBus>) {
    let result = tauri::async_runtime::spawn_blocking(|| {
        let registry_dir = workspace_paths::registry_dir();
        let prev_engines_etag = plugin_registry::read_etag(&registry_dir, "engines");
        let prev_models_etag = plugin_registry::read_etag(&registry_dir, "models");

        // Conditional GET (If-None-Match) means the common case — the registry hasn't
        // changed since the last poll — costs a cheap 304 response instead of
        // re-downloading, re-validating, and re-writing the full registry every 6
        // hours for no reason.
        let poll = plugin_registry::fetch_remote_registry_conditional(
            &app_config().registry_poll_url,
            prev_engines_etag.as_deref(),
            prev_models_etag.as_deref(),
        )?;

        if poll.engines.is_none() && poll.models.is_none() {
            return Ok::<bool, plugin_registry::PluginRegistryError>(false);
        }

        // apply_remote_registry validates+writes both files as a pair, so an unchanged
        // file's current on-disk content is read back in as its "new" content —
        // re-validating and re-writing it is a no-op, and keeps the pair-atomic
        // guarantee rather than special-casing a single-file update.
        let engines_json = match poll.engines {
            Some(json) => json,
            None => read_current(&registry_dir, "engines.json")?,
        };
        let models_json = match poll.models {
            Some(json) => json,
            None => read_current(&registry_dir, "models.json")?,
        };

        // Validate (host-allowlisted URLs, safe library_file paths) and atomically
        // persist only if the whole fetched pair passes — a poll that fails
        // validation, or a network/parse error, leaves the last-known-good registry
        // files exactly as they were, so a compromised or malformed response can
        // never partially or fully overwrite what's already trusted.
        // Signature verification is wired end to end (see plugin_registry::signing)
        // but not yet enabled here: no real Ed25519 keypair has been provisioned or
        // wired into a publish pipeline for the actual registryPollUrl yet, so
        // requiring one today would just break every real poll. Pass real
        // signatures once `examples/sign_registry.rs` is run against a real
        // publish and the resulting public key is set in config/app.json.
        plugin_registry::apply_remote_registry(
            &registry_dir,
            &engines_json,
            &models_json,
            &app_config().registry_allowed_hosts,
            None,
        )?;

        if let Some(etag) = &poll.engines_etag {
            let _ = plugin_registry::write_etag(&registry_dir, "engines", etag);
        }
        if let Some(etag) = &poll.models_etag {
            let _ = plugin_registry::write_etag(&registry_dir, "models", etag);
        }

        Ok(true)
    })
    .await;
    match result {
        Ok(Ok(true)) => {
            tracing::info!("polled and applied the remote plugin registry");
            event_bus.publish(DaemonEvent::RegistryPolled { ok: true });
        }
        Ok(Ok(false)) => {
            tracing::info!("polled the remote plugin registry: not modified");
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

fn read_current(
    registry_dir: &std::path::Path,
    filename: &str,
) -> Result<String, plugin_registry::PluginRegistryError> {
    std::fs::read_to_string(registry_dir.join(filename)).map_err(|source| {
        plugin_registry::PluginRegistryError::Io {
            path: registry_dir.join(filename),
            source,
        }
    })
}
