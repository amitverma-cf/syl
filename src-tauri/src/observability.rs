use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use sysinfo::System;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu_usage_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub process_memory_bytes: u64,
    pub workspace_disk_bytes: u64,
}

#[derive(Default)]
pub struct ObservabilityState {
    cpu_usage_percent_x1000: AtomicU64,
    memory_used_bytes: AtomicU64,
    memory_total_bytes: AtomicU64,
    process_memory_bytes: AtomicU64,
    workspace_disk_bytes: AtomicU64,
}

impl ObservabilityState {
    pub fn snapshot(&self) -> SystemStats {
        SystemStats {
            cpu_usage_percent: self.cpu_usage_percent_x1000.load(Ordering::Relaxed) as f32 / 1000.0,
            memory_used_bytes: self.memory_used_bytes.load(Ordering::Relaxed),
            memory_total_bytes: self.memory_total_bytes.load(Ordering::Relaxed),
            process_memory_bytes: self.process_memory_bytes.load(Ordering::Relaxed),
            workspace_disk_bytes: self.workspace_disk_bytes.load(Ordering::Relaxed),
        }
    }
}

pub fn available_memory_bytes() -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.available_memory()
}

pub fn spawn_sampler(state: Arc<ObservabilityState>) {
    tauri::async_runtime::spawn(async move {
        let mut sys = System::new_all();
        let pid = sysinfo::get_current_pid().ok();

        loop {
            sys.refresh_all();

            let cpu_usage_percent = sys.global_cpu_usage();
            let memory_used_bytes = sys.used_memory();
            let memory_total_bytes = sys.total_memory();
            let process_memory_bytes = pid
                .and_then(|pid| sys.process(pid))
                .map(|process| process.memory())
                .unwrap_or(0);
            let workspace_disk_bytes = dir_size(&syl_core::workspace_paths::workspace_root());

            state
                .cpu_usage_percent_x1000
                .store((cpu_usage_percent * 1000.0) as u64, Ordering::Relaxed);
            state
                .memory_used_bytes
                .store(memory_used_bytes, Ordering::Relaxed);
            state
                .memory_total_bytes
                .store(memory_total_bytes, Ordering::Relaxed);
            state
                .process_memory_bytes
                .store(process_memory_bytes, Ordering::Relaxed);
            state
                .workspace_disk_bytes
                .store(workspace_disk_bytes, Ordering::Relaxed);

            tracing::info!(
                cpu_usage_percent,
                memory_used_bytes,
                memory_total_bytes,
                process_memory_bytes,
                workspace_disk_bytes,
                "system stats sampled"
            );

            tokio::time::sleep(SAMPLE_INTERVAL).await;
        }
    });
}

fn dir_size(path: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(metadata) if metadata.is_dir() => dir_size(&entry.path()),
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        })
        .sum()
}

#[tauri::command]
pub fn system_stats(state: tauri::State<'_, Arc<ObservabilityState>>) -> SystemStats {
    state.snapshot()
}
