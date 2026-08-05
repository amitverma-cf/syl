use std::time::Duration;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::MakeWriterExt;

const LOG_RETENTION: Duration = Duration::from_secs(14 * 24 * 60 * 60);

pub fn init() -> WorkerGuard {
    let log_dir = syl_core::workspace_paths::logs_dir();
    prune_old_logs(&log_dir);

    let file_appender = tracing_appender::rolling::daily(&log_dir, "syl.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stdout.and(non_blocking))
        .init();

    tracing::info!(dir = %log_dir.display(), "logging initialized");
    guard
}

fn prune_old_logs(log_dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for entry in entries.filter_map(|e| e.ok()) {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if now.duration_since(modified).unwrap_or_default() > LOG_RETENTION {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
