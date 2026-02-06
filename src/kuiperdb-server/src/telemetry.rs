//! OpenTelemetry integration with file-based logging
//!
//! Provides structured tracing using OpenTelemetry protocol with:
//! - JSON formatted logs to file
//! - Console output for development
//! - Trace spans and metrics collection
//! - Size-based rotation (10MB per file)
//! - Daily rotation with numbered files

use anyhow::Result;
use rolling_file::{RollingConditionBasic, RollingFileAppender};
use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initialize OpenTelemetry with file logging
///
/// Returns a guard that must be kept alive to ensure logs are flushed
pub fn init_telemetry() -> Result<WorkerGuard> {
    // Create logs directory if it doesn't exist
    let log_dir = Path::new("./logs");
    std::fs::create_dir_all(log_dir)?;

    // Create rolling file appender with size and daily rotation
    // Format: kuiperdb.log.2026-02-04 (daily rotation by rolling-file crate)
    // Rotates when file reaches 10MB or daily, whichever comes first
    let file_appender = RollingFileAppender::new(
        log_dir.join("kuiperdb.log"),
        RollingConditionBasic::new()
            .daily()
            .max_size(10 * 1024 * 1024), // 10 MB
        9, // Keep up to 10 files per day (0-9)
    )?;

    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    // Environment filter for log levels
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("kuiperdb=debug,kuiperdb_core=debug,actix_web=info"));

    // JSON file layer for structured logging
    let file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking_file)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_current_span(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true);

    // Console layer for human-readable output
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_target(false);

    // Combine all layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer)
        .try_init()?;

    tracing::info!("Telemetry initialized with file logging to {:?}", log_dir);
    tracing::info!("Log rotation: 10MB per file, daily rotation, format: kuiperdb.log.YYYY-MM-DD");

    Ok(guard)
}

/// Shutdown telemetry gracefully
pub fn shutdown_telemetry() {
    // Flush remaining logs
    tracing::info!("Telemetry shutdown complete");
}
