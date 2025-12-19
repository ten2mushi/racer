use std::fs;
use std::path::PathBuf;

use file_rotate::{
    compression::Compression,
    suffix::AppendCount,
    ContentLimit, FileRotate,
};
use tracing_subscriber::{
    fmt::{self, MakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub log_dir: PathBuf,
    pub max_size_mb: u64,
    pub max_files: usize,
}

fn make_rotating_writer(
    path: PathBuf,
    max_size_mb: u64,
    max_files: usize,
) -> FileRotate<AppendCount> {
    FileRotate::new(
        path,
        AppendCount::new(max_files),
        ContentLimit::Bytes((max_size_mb * 1024 * 1024) as usize),
        Compression::None,
        #[cfg(unix)]
        None,
    )
}

struct RotatingWriter {
    writer: std::sync::Arc<std::sync::Mutex<FileRotate<AppendCount>>>,
}

impl RotatingWriter {
    fn new(rotate: FileRotate<AppendCount>) -> Self {
        Self {
            writer: std::sync::Arc::new(std::sync::Mutex::new(rotate)),
        }
    }
}

impl<'a> MakeWriter<'a> for RotatingWriter {
    type Writer = RotatingWriterGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        RotatingWriterGuard {
            guard: self.writer.lock().unwrap(),
        }
    }
}

struct RotatingWriterGuard<'a> {
    guard: std::sync::MutexGuard<'a, FileRotate<AppendCount>>,
}

impl<'a> std::io::Write for RotatingWriterGuard<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.guard.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.guard.flush()
    }
}

pub fn init_logging(config: LoggingConfig) -> anyhow::Result<()> {
    fs::create_dir_all(&config.log_dir)?;

    let messages_writer = RotatingWriter::new(make_rotating_writer(
        config.log_dir.join("messages.jsonl"),
        config.max_size_mb,
        config.max_files,
    ));

    let events_writer = RotatingWriter::new(make_rotating_writer(
        config.log_dir.join("events.jsonl"),
        config.max_size_mb,
        config.max_files,
    ));

    let protocol_writer = RotatingWriter::new(make_rotating_writer(
        config.log_dir.join("protocol.jsonl"),
        config.max_size_mb,
        config.max_files,
    ));

    let console_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_filter(EnvFilter::from_default_env().add_directive("racer=info".parse()?));

    let messages_layer = fmt::layer()
        .json()
        .with_writer(messages_writer)
        .with_filter(EnvFilter::new("racer::protocol::messages=trace"));

    let events_layer = fmt::layer()
        .json()
        .with_writer(events_writer)
        .with_filter(EnvFilter::new("racer::node=info,racer::network=info"));

    let protocol_layer = fmt::layer()
        .json()
        .with_writer(protocol_writer)
        .with_filter(EnvFilter::new("racer::protocol::gossip=trace,racer::plato=debug"));

    tracing_subscriber::registry()
        .with(console_layer)
        .with(messages_layer)
        .with(events_layer)
        .with(protocol_layer)
        .init();

    tracing::info!(
        log_dir = %config.log_dir.display(),
        max_size_mb = config.max_size_mb,
        max_files = config.max_files,
        "Logging initialized"
    );

    Ok(())
}
