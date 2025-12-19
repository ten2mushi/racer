use serde::{Deserialize, Serialize};

use super::ConfigError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatoConfig {
    #[serde(default = "default_target_latency")]
    pub target_latency_secs: f64,
    #[serde(default = "default_publishing_frequency")]
    pub target_publishing_frequency_secs: f64,
    #[serde(default = "default_max_publishing_frequency")]
    pub max_publishing_frequency_secs: f64,
    #[serde(default = "default_minimum_latency")]
    pub minimum_latency_secs: f64,
    #[serde(default = "default_max_gossip_timeout")]
    pub max_gossip_timeout_secs: f64,
    #[serde(default = "default_rsi_increase_period")]
    pub rsi_increase_period: usize,
    #[serde(default = "default_rsi_decrease_period")]
    pub rsi_decrease_period: usize,
    #[serde(default = "default_rsi_overbought")]
    pub rsi_overbought: f64,
    #[serde(default = "default_rsi_oversold")]
    pub rsi_oversold: f64,
    #[serde(default = "default_own_latency_weight")]
    pub own_latency_weight: f64,
    #[serde(default = "default_savgol_increase_window")]
    pub savgol_increase_window: usize,
    #[serde(default = "default_savgol_decrease_window")]
    pub savgol_decrease_window: usize,
}

fn default_target_latency() -> f64 {
    2.5
}

fn default_publishing_frequency() -> f64 {
    2.5
}

fn default_max_publishing_frequency() -> f64 {
    10.0
}

fn default_minimum_latency() -> f64 {
    1.0
}

fn default_max_gossip_timeout() -> f64 {
    60.0
}

fn default_rsi_increase_period() -> usize {
    14
}

fn default_rsi_decrease_period() -> usize {
    21
}

fn default_rsi_overbought() -> f64 {
    70.0
}

fn default_rsi_oversold() -> f64 {
    30.0
}

fn default_own_latency_weight() -> f64 {
    0.6
}

fn default_savgol_increase_window() -> usize {
    14
}

fn default_savgol_decrease_window() -> usize {
    21
}

impl PlatoConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.minimum_latency_secs <= 0.0 {
            return Err(ConfigError::Validation(
                "minimum_latency_secs must be positive".into(),
            ));
        }

        if self.target_latency_secs < self.minimum_latency_secs {
            return Err(ConfigError::Validation(
                "target_latency_secs must be >= minimum_latency_secs".into(),
            ));
        }

        if self.max_gossip_timeout_secs <= self.target_latency_secs {
            return Err(ConfigError::Validation(
                "max_gossip_timeout_secs must be > target_latency_secs".into(),
            ));
        }

        if !(0.0..=1.0).contains(&self.own_latency_weight) {
            return Err(ConfigError::Validation(
                "own_latency_weight must be between 0.0 and 1.0".into(),
            ));
        }

        if self.rsi_overbought <= self.rsi_oversold {
            return Err(ConfigError::Validation(
                "rsi_overbought must be > rsi_oversold".into(),
            ));
        }

        Ok(())
    }
}

impl Default for PlatoConfig {
    fn default() -> Self {
        Self {
            target_latency_secs: default_target_latency(),
            target_publishing_frequency_secs: default_publishing_frequency(),
            max_publishing_frequency_secs: default_max_publishing_frequency(),
            minimum_latency_secs: default_minimum_latency(),
            max_gossip_timeout_secs: default_max_gossip_timeout(),
            rsi_increase_period: default_rsi_increase_period(),
            rsi_decrease_period: default_rsi_decrease_period(),
            rsi_overbought: default_rsi_overbought(),
            rsi_oversold: default_rsi_oversold(),
            own_latency_weight: default_own_latency_weight(),
            savgol_increase_window: default_savgol_increase_window(),
            savgol_decrease_window: default_savgol_decrease_window(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_valid() {
        let config = PlatoConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_latency() {
        let config = PlatoConfig {
            minimum_latency_secs: 0.0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }
}
