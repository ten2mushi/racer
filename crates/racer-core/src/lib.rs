//! # racer-core
//!
//! Core types and traits for the RACER consensus protocol.
//!
//! This crate provides:
//! - [`Message`] trait for custom consensus payloads
//! - [`ValidationError`] for field validation
//! - Common error types

pub mod error;
pub mod message;
pub mod validation;

pub use error::RacerError;
pub use message::Message;
pub use validation::{FieldValidator, ValidationError, ValidationResult};
