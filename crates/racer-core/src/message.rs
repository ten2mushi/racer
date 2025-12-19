use serde::{de::DeserializeOwned, Serialize};

use crate::validation::ValidationResult;

/// Trait for consensus message payloads.
///
/// All messages sent through RACER must implement this trait.
/// The [`racer_macros::racer_message`] proc macro can generate
/// implementations from TOML configuration files.
///
/// # Example
///
/// ```rust
/// use racer_core::Message;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Clone, Debug, Serialize, Deserialize)]
/// pub struct SensorReading {
///     pub timestamp: u64,
///     pub value: f64,
/// }
///
/// impl Message for SensorReading {
///     fn id(&self) -> u64 {
///         self.timestamp
///     }
/// }
/// ```
pub trait Message: Clone + Send + Sync + Serialize + DeserializeOwned + 'static {
    fn id(&self) -> u64;

    fn merkle_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    fn validate(&self) -> ValidationResult {
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, serde::Deserialize)]
pub struct DefaultMessage {
    pub timestamp: u64,
    pub padding: u64,
}

impl DefaultMessage {
    pub fn new() -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            padding: 0,
        }
    }

    pub fn with_padding(padding: u64) -> Self {
        let mut msg = Self::new();
        msg.padding = padding;
        msg
    }
}

impl Message for DefaultMessage {
    fn id(&self) -> u64 {
        self.timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_message() {
        let msg = DefaultMessage::new();
        assert!(msg.id() > 0);
        assert_eq!(msg.padding, 0);
    }

    #[test]
    fn test_merkle_bytes() {
        let msg = DefaultMessage::with_padding(42);
        let bytes = msg.merkle_bytes();
        assert!(!bytes.is_empty());

        let parsed: DefaultMessage = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.padding, 42);
    }
}
