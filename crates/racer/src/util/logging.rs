use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use tokio::sync::mpsc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_log_dir")]
    pub log_dir: String,

    #[serde(default = "default_delivered_file")]
    pub delivered_file: String,

    #[serde(default)]
    pub rotation_enabled: bool,

    #[serde(default = "default_max_size")]
    pub max_file_size_mb: u64,

    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_enabled() -> bool {
    true
}

fn default_log_dir() -> String {
    "logs/{node_id}".into()
}

fn default_delivered_file() -> String {
    "delivered.jsonl".into()
}

fn default_max_size() -> u64 {
    100
}

fn default_max_files() -> usize {
    5
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            log_dir: default_log_dir(),
            delivered_file: default_delivered_file(),
            rotation_enabled: false,
            max_file_size_mb: default_max_size(),
            max_files: default_max_files(),
        }
    }
}

impl LogConfig {
    pub fn resolve_log_dir(&self, node_id: &str) -> PathBuf {
        PathBuf::from(self.log_dir.replace("{node_id}", node_id))
    }

    pub fn delivered_path(&self, node_id: &str) -> PathBuf {
        self.resolve_log_dir(node_id).join(&self.delivered_file)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DeliveredEntry {
    pub seq: u64,
    pub batch_id: String,
    pub creator: String,
    pub merkle_root: String,
    pub batch_size: usize,
    pub delivered_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

enum LogMessage {
    Entry(String),
    Shutdown,
}

#[derive(Clone)]
pub struct DeliveredMessageLogger {
    sender: mpsc::UnboundedSender<LogMessage>,
    seq: std::sync::Arc<AtomicU64>,
}

impl DeliveredMessageLogger {
    pub fn new(config: &LogConfig, node_id: &str) -> Option<Self> {
        if !config.enabled {
            return None;
        }

        let log_dir = config.resolve_log_dir(node_id);
        let log_path = config.delivered_path(node_id);

        if let Err(e) = fs::create_dir_all(&log_dir) {
            tracing::warn!(path = %log_dir.display(), error = %e, "failed to create log directory");
            return None;
        }

        let file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(path = %log_path.display(), error = %e, "failed to open log file");
                return None;
            }
        };

        let (sender, receiver) = mpsc::unbounded_channel();
        let seq = std::sync::Arc::new(AtomicU64::new(0));

        let path_for_task = log_path.clone();
        tokio::spawn(async move {
            writer_task(receiver, file, path_for_task).await;
        });

        tracing::info!(path = %log_path.display(), "delivered message logger initialized");

        Some(Self { sender, seq })
    }

    pub fn log<M: Serialize>(&self, batch_id: &str, creator: &str, merkle_root: &str, batch_size: usize, messages: &[M]) {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;

        let payload = if messages.len() == 1 {
            serde_json::to_value(&messages[0]).ok()
        } else {
            serde_json::to_value(messages).ok()
        };

        let entry = DeliveredEntry {
            seq,
            batch_id: batch_id.to_string(),
            creator: creator.to_string(),
            merkle_root: merkle_root.to_string(),
            batch_size,
            delivered_at: chrono_now(),
            payload,
        };

        let line = match serde_json::to_string(&entry) {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!(error = %e, "failed to serialize log entry");
                return;
            }
        };

        if self.sender.send(LogMessage::Entry(line)).is_err() {
            tracing::warn!(seq, "log channel closed, entry dropped");
        }
    }

    pub fn current_seq(&self) -> u64 {
        self.seq.load(Ordering::SeqCst)
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(LogMessage::Shutdown);
    }
}

async fn writer_task(
    mut receiver: mpsc::UnboundedReceiver<LogMessage>,
    file: File,
    path: PathBuf,
) {
    let mut writer = BufWriter::new(file);

    while let Some(msg) = receiver.recv().await {
        match msg {
            LogMessage::Entry(line) => {
                if let Err(e) = writeln!(writer, "{}", line) {
                    tracing::warn!(path = %path.display(), error = %e, "failed to write log entry");
                }
                if let Err(e) = writer.flush() {
                    tracing::warn!(path = %path.display(), error = %e, "failed to flush log");
                }
            }
            LogMessage::Shutdown => {
                let _ = writer.flush();
                tracing::debug!(path = %path.display(), "log writer shutting down");
                break;
            }
        }
    }
}

fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "{}.{:03}Z",
        now.as_secs(),
        now.subsec_millis()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::default();
        assert!(config.enabled);
        assert_eq!(config.log_dir, "logs/{node_id}");
        assert_eq!(config.delivered_file, "delivered.jsonl");
    }

    #[test]
    fn test_path_templating() {
        let config = LogConfig::default();
        let path = config.resolve_log_dir("sensor-0");
        assert_eq!(path, PathBuf::from("logs/sensor-0"));
    }

    #[tokio::test]
    async fn test_logger_writes_entries() {
        let dir = tempdir().unwrap();
        let config = LogConfig {
            enabled: true,
            log_dir: dir.path().to_string_lossy().to_string(),
            delivered_file: "test.jsonl".into(),
            ..Default::default()
        };

        let logger = DeliveredMessageLogger::new(&config, "test-node").unwrap();

        logger.log::<String>("batch-1", "creator-1", "merkle-abc", 1, &["test message".to_string()]);

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        logger.shutdown();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let contents = std::fs::read_to_string(dir.path().join("test.jsonl")).unwrap();
        assert!(contents.contains("batch-1"));
        assert!(contents.contains("creator-1"));
        assert!(contents.contains("test message"));
    }
}
