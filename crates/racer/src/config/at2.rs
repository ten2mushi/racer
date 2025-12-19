use serde::{Deserialize, Serialize};

use super::ConfigError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct At2Config {
    #[serde(default = "default_sample_size")]
    pub echo_sample_size: usize,
    #[serde(default = "default_sample_size")]
    pub ready_sample_size: usize,
    #[serde(default = "default_sample_size")]
    pub delivery_sample_size: usize,
    #[serde(default = "default_ready_threshold")]
    pub ready_threshold: usize,
    #[serde(default = "default_feedback_threshold")]
    pub feedback_threshold: usize,
    #[serde(default = "default_delivery_threshold")]
    pub delivery_threshold: usize,
}

fn default_sample_size() -> usize {
    6
}

fn default_ready_threshold() -> usize {
    4
}

fn default_feedback_threshold() -> usize {
    5
}

fn default_delivery_threshold() -> usize {
    6
}

impl At2Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(self.ready_threshold < self.feedback_threshold
            && self.feedback_threshold < self.delivery_threshold)
        {
            return Err(ConfigError::Validation(
                "thresholds must satisfy: ready < feedback < delivery".into(),
            ));
        }

        let min_ready = (self.echo_sample_size / 2) + 1;
        if self.ready_threshold < min_ready {
            return Err(ConfigError::Validation(format!(
                "ready_threshold ({}) must be >= {} (majority of echo_sample_size)",
                self.ready_threshold, min_ready
            )));
        }

        let min_feedback = (self.ready_sample_size * 3 + 3) / 4; // ceil(n * 0.75)
        if self.feedback_threshold < min_feedback {
            return Err(ConfigError::Validation(format!(
                "feedback_threshold ({}) must be >= {} (75% of ready_sample_size)",
                self.feedback_threshold, min_feedback
            )));
        }

        let min_delivery = (self.delivery_sample_size * 85 + 99) / 100; // ceil(n * 0.85)
        if self.delivery_threshold < min_delivery {
            return Err(ConfigError::Validation(format!(
                "delivery_threshold ({}) must be >= {} (85% of delivery_sample_size)",
                self.delivery_threshold, min_delivery
            )));
        }

        Ok(())
    }

    pub fn with_sample_size(size: usize) -> Self {
        let ready = (size / 2) + 1;
        let feedback = (size * 3 + 3) / 4;
        let delivery = (size * 85 + 99) / 100;

        Self {
            echo_sample_size: size,
            ready_sample_size: size,
            delivery_sample_size: size,
            ready_threshold: ready,
            feedback_threshold: feedback.max(ready + 1),
            delivery_threshold: delivery.max(feedback + 1).max(ready + 2),
        }
    }
}

impl Default for At2Config {
    fn default() -> Self {
        Self {
            echo_sample_size: default_sample_size(),
            ready_sample_size: default_sample_size(),
            delivery_sample_size: default_sample_size(),
            ready_threshold: default_ready_threshold(),
            feedback_threshold: default_feedback_threshold(),
            delivery_threshold: default_delivery_threshold(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_valid() {
        let config = At2Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_with_sample_size() {
        for size in [3, 6, 10, 20, 100] {
            let config = At2Config::with_sample_size(size);
            assert!(
                config.validate().is_ok(),
                "failed for size {}: {:?}",
                size,
                config
            );
        }
    }

    #[test]
    fn test_invalid_ordering() {
        let config = At2Config {
            ready_threshold: 6,
            feedback_threshold: 5,
            delivery_threshold: 7,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }
}
