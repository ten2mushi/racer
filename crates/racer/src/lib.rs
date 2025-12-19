//! # RACER
//!
//! Lightweight leaderless consensus for IoT networks.
//!
//! RACER implements the SPDE (Sequenced Probabilistic Double Echo) consensus
//! protocol with PLATO (Peer-assisted Latency-Aware Traffic Optimisation)
//! congestion control.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use racer::prelude::*;
//!
//! // Define a message type from TOML
//! #[racer_message("examples/config/default_message.toml")]
//! pub struct SensorReading;
//!
//! // Or use the built-in DefaultMessage
//! use racer_core::message::DefaultMessage;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration
//!     let config = RacerConfig::from_file("racer.toml")?;
//!     
//!     // Create and start node
//!     let node = Node::<DefaultMessage>::new(config).await?;
//!     node.start().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! - `bls`: Enable BLS signature aggregation (requires `blst` C library)
//! - `cli`: Enable CLI binary with logging and key generation

pub mod config;
pub mod crypto;
pub mod network;
pub mod plato;
pub mod protocol;
pub mod node;
pub mod util;

#[cfg(feature = "cli")]
pub mod cli;

pub use racer_core::{Message, RacerError, ValidationError};

pub mod prelude {
    pub use racer_core::Message;
    pub use racer_macros::racer_message;

    pub use crate::config::RacerConfig;
    pub use crate::node::Node;
}

