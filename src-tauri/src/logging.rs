use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::MakeWriterExt;

pub fn init() -> WorkerGuard {
    let log_dir = core_types::workspace_paths::logs_dir();
    let file_appender = tracing_appender::rolling::daily(&log_dir, "syl.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stdout.and(non_blocking))
        .init();

    tracing::info!(dir = %log_dir.display(), "logging initialized");
    guard
}
