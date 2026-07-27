//! Structured logging setup: human-readable output to the console, plus a persisted,
//! daily-rotated log file so behavior can be inspected after the app window has closed.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::MakeWriterExt;

/// Initializes the global tracing subscriber. The returned [`WorkerGuard`] must be kept alive
/// for the lifetime of the app — dropping it stops the background thread that flushes log
/// lines to disk, silently truncating anything not yet written.
pub fn init() -> WorkerGuard {
    let log_dir = log_directory();
    let file_appender = tracing_appender::rolling::daily(&log_dir, "syl.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stdout.and(non_blocking))
        .init();

    tracing::info!(dir = %log_dir.display(), "logging initialized");
    guard
}

fn log_directory() -> std::path::PathBuf {
    directories::ProjectDirs::from("com", "syl", "syl")
        .map(|dirs| dirs.data_dir().join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"))
}
