mod at2;
mod plato;

use std::path::Path;

use serde::{Deserialize, Serialize};

pub use at2::At2Config;
pub use plato::PlatoConfig;
pub use crate::util::logging::LogConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacerConfig {
    pub node: NodeConfig,
    pub consensus: At2Config,
    pub plato: PlatoConfig,
    pub peers: PeerConfig,
    #[serde(default)]
    pub logging: LogConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub id: Option<String>,
    #[serde(default = "default_router_bind")]
    pub router_bind: String,
    #[serde(default = "default_publisher_bind")]
    pub publisher_bind: String,
    #[serde(default)]
    pub selection_type: SelectionType,
}

fn default_router_bind() -> String {
    "tcp://0.0.0.0:20001".into()
}

fn default_publisher_bind() -> String {
    "tcp://0.0.0.0:21001".into()
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SelectionType {
    #[default]
    Normal,
    Random,
    Poisson,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    #[serde(default)]
    pub routers: Vec<String>,
}

impl RacerConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::Io(e.to_string()))?;
        Self::from_toml(&content)
    }

    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(content)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.consensus.validate()?;
        self.plato.validate()?;
        Ok(())
    }

    pub fn minimal() -> Self {
        Self {
            node: NodeConfig {
                id: None,
                router_bind: default_router_bind(),
                publisher_bind: default_publisher_bind(),
                selection_type: SelectionType::Normal,
            },
            consensus: At2Config::default(),
            plato: PlatoConfig::default(),
            peers: PeerConfig { routers: vec![] },
            logging: LogConfig::default(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("validation error: {0}")]
    Validation(String),
}

impl Default for RacerConfig {
    fn default() -> Self {
        Self::minimal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_config() {
        let config = RacerConfig::minimal();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_parse_toml() {
        let toml = r#"
            [node]
            router_bind = "tcp://0.0.0.0:20001"
            publisher_bind = "tcp://0.0.0.0:21001"
            selection_type = "normal"

            [consensus]
            echo_sample_size = 6
            ready_sample_size = 6
            delivery_sample_size = 6
            ready_threshold = 4
            feedback_threshold = 5
            delivery_threshold = 6

            [plato]
            target_latency_secs = 2.5

            [peers]
            routers = ["tcp://192.168.1.10:20001"]
        "#;

        let config = RacerConfig::from_toml(toml).unwrap();
        assert_eq!(config.consensus.echo_sample_size, 6);
        assert_eq!(config.peers.routers.len(), 1);
    }
}
