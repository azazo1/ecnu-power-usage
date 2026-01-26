use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use tokio::fs;

/// 初始化日志输出
///
/// 使用 RUST_LOG 控制日志程度.
pub(crate) async fn init(log_dir: impl AsRef<Path>) -> crate::Result<WorkerGuard> {
    let log_dir = log_dir.as_ref();
    if !log_dir.exists() {
        fs::create_dir(&log_dir).await?;
    }
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "server.log");
    let (logging_appender, guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(logging_appender)
        .with_ansi(false);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stdout_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();
    Ok(guard)
}
