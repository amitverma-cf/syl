use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use core_types::workspace_paths;
use daemon::events::DaemonEvent;
use daemon::jobs::{
    add_scheduled_job as persist_add_job, load_scheduled_jobs,
    remove_scheduled_job as persist_remove_job, ScheduledJob,
};
use daemon::scheduler::{CronScheduler, Uuid};
use memory::ConversationStore;
use tauri::{AppHandle, Manager};

use crate::commands::{run_generate, run_generate_cloud};
use crate::daemon::DaemonState;
use crate::flows::{FlowState, DEFAULT_FLOW_NAME};
use crate::{AppState, ToolState};

/// Holds the live cron scheduler plus a map from a persisted job's stable id to the scheduler's
/// own runtime id, so `remove_scheduled_job` knows which live job to unregister.
pub struct SchedulerState {
    pub scheduler: Arc<CronScheduler>,
    running: Mutex<HashMap<String, Uuid>>,
}

impl SchedulerState {
    pub fn new(scheduler: Arc<CronScheduler>) -> Self {
        Self {
            scheduler,
            running: Mutex::new(HashMap::new()),
        }
    }
}

/// Loads every persisted scheduled job and registers it with `scheduler`, tracking each one's
/// live runtime id in `scheduler_state`. Called once at startup — independent of whether the
/// main window is open, same as the registry-poll job.
pub async fn register_persisted_jobs(app: &AppHandle, scheduler_state: &SchedulerState) {
    let jobs_file = workspace_paths::scheduled_jobs_file();
    for job in load_scheduled_jobs(&jobs_file) {
        let job_name = job.name.clone();
        match register_job(app, &scheduler_state.scheduler, job.clone()).await {
            Ok(job_uuid) => {
                scheduler_state
                    .running
                    .lock()
                    .unwrap()
                    .insert(job.id.clone(), job_uuid);
            }
            Err(err) => {
                tracing::error!(?err, job = %job_name, "failed to register scheduled job");
            }
        }
    }
}

async fn register_job(
    app: &AppHandle,
    scheduler: &CronScheduler,
    job: ScheduledJob,
) -> Result<Uuid, tokio_cron_scheduler::JobSchedulerError> {
    let app = app.clone();
    let cron_expr = job.cron_expr.clone();
    scheduler
        .add_cron_job(
            &cron_expr,
            Arc::new(move || {
                let app = app.clone();
                let job = job.clone();
                Box::pin(fire_job(app, job)) as Pin<Box<dyn Future<Output = ()> + Send>>
            }),
        )
        .await
}

/// Runs one unattended chat turn for `job`: the exact same tool-calling loop and flow FSM a
/// live chat message goes through, just without a UI `Channel` to stream pieces to.
async fn fire_job(app: AppHandle, job: ScheduledJob) {
    let app_state = app.state::<AppState>();
    let tool_state = app.state::<ToolState>();
    let flow_state = app.state::<FlowState>();
    let daemon_state = app.state::<DaemonState>();

    // Idempotent: the conversation is created once, on the job's first-ever firing.
    let _ = app_state.conversation_store.create_conversation(
        &job.conversation_id,
        &job.name,
        DEFAULT_FLOW_NAME,
    );

    let result = fire_turn(&app_state, &tool_state, &flow_state, &job).await;

    if result.is_ok() {
        if let Some(info) = flow_state.advance(&job.conversation_id, "message") {
            daemon_state
                .event_bus
                .publish(DaemonEvent::FlowStateChanged {
                    flow: info.flow_name,
                    state: info.state_name,
                });
        }
    } else if let Err(message) = &result {
        tracing::error!(%message, job = %job.name, "scheduled job firing failed");
    }
    daemon_state
        .event_bus
        .publish(DaemonEvent::ScheduledJobFired {
            job: job.name.clone(),
            ok: result.is_ok(),
        });
}

async fn fire_turn(
    app_state: &tauri::State<'_, AppState>,
    tool_state: &tauri::State<'_, ToolState>,
    flow_state: &tauri::State<'_, FlowState>,
    job: &ScheduledJob,
) -> Result<(), String> {
    let store = app_state.conversation_store.clone();
    let flow_turn = flow_state.ensure_and_take_turn(&job.conversation_id)?;
    let tools = tool_state
        .executor
        .tool_specs_filtered(&flow_turn.tool_allowlist);

    match &job.model {
        Some(model_id) => {
            run_generate_cloud(
                &store,
                &job.conversation_id,
                &job.prompt,
                &flow_turn.system_prompt,
                model_id,
                &tools,
                &tool_state.executor,
                |_piece| {},
            )
            .await
        }
        None => {
            let store = store.clone();
            let conversation_id = job.conversation_id.clone();
            let prompt = job.prompt.clone();
            let system_prompt = flow_turn.system_prompt.clone();
            let executor = tool_state.executor.clone();
            tauri::async_runtime::spawn_blocking(move || {
                run_generate(
                    &store,
                    &conversation_id,
                    &prompt,
                    &system_prompt,
                    &tools,
                    &executor,
                    |_piece| {},
                )
            })
            .await
            .map_err(|e| e.to_string())?
        }
    }
}

#[tauri::command]
pub fn list_scheduled_jobs() -> Vec<ScheduledJob> {
    load_scheduled_jobs(&workspace_paths::scheduled_jobs_file())
}

#[tauri::command]
pub async fn add_scheduled_job(
    name: String,
    cron_expr: String,
    prompt: String,
    model: Option<String>,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    scheduler_state: tauri::State<'_, SchedulerState>,
) -> Result<ScheduledJob, String> {
    let conversation_id = Uuid::new_v4().to_string();
    state
        .conversation_store
        .create_conversation(&conversation_id, &name, DEFAULT_FLOW_NAME)
        .map_err(|e| e.to_string())?;

    let job = ScheduledJob {
        id: Uuid::new_v4().to_string(),
        name,
        cron_expr,
        conversation_id,
        prompt,
        model,
    };
    let job =
        persist_add_job(&workspace_paths::scheduled_jobs_file(), job).map_err(|e| e.to_string())?;

    let job_uuid = register_job(&app, &scheduler_state.scheduler, job.clone())
        .await
        .map_err(|e| e.to_string())?;
    scheduler_state
        .running
        .lock()
        .unwrap()
        .insert(job.id.clone(), job_uuid);

    Ok(job)
}

#[tauri::command]
pub async fn remove_scheduled_job(
    id: String,
    scheduler_state: tauri::State<'_, SchedulerState>,
) -> Result<(), String> {
    let job_uuid = scheduler_state.running.lock().unwrap().remove(&id);
    if let Some(job_uuid) = job_uuid {
        scheduler_state
            .scheduler
            .remove_job(&job_uuid)
            .await
            .map_err(|e| e.to_string())?;
    }
    persist_remove_job(&workspace_paths::scheduled_jobs_file(), &id).map_err(|e| e.to_string())
}
