//! # racer-macros
//!
//! Procedural macros for generating RACER message types from TOML configuration.
//!
//! # Usage
//!
//! ```ignore
//! use racer::prelude::*;
//!
//! #[racer_message("examples/config/sensor.toml")]
//! pub struct SensorReading;
//! ```
//!
//! The macro reads the TOML file at compile-time and generates:
//! - Struct fields based on the configuration
//! - `Message` trait implementation
//! - Validation logic for field constraints

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemStruct, LitStr};

mod codegen;
mod parser;
mod types;

/// Generates a message struct from a TOML configuration file.
///
/// # Arguments
///
/// The attribute takes a single string literal: the path to the TOML file
/// relative to the crate's `Cargo.toml`.
///
/// # TOML Schema
///
/// ```toml
/// [message]
/// name = "SensorReading"
///
/// [[message.fields]]
/// name = "timestamp"
/// type = "u64"
/// id_field = true       # Used for Message::id()
///
/// [[message.fields]]
/// name = "value"
/// type = "f64"
/// min = 0.0
/// max = 100.0
/// required = true
/// ```
///
/// # Supported Types
///
/// - Primitives: `u8`-`u64`, `i8`-`i64`, `f32`, `f64`, `bool`, `string`, `bytes`
/// - Collections: `array<T>`, `map<K, V>`
///
/// # Validation Attributes
///
/// - `required`: Field cannot be empty
/// - `min` / `max`: Numeric range validation
/// - `min_length` / `max_length`: Length bounds for strings/arrays
#[proc_macro_attribute]
pub fn racer_message(attr: TokenStream, item: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemStruct);

    match codegen::generate(&path_lit, &input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
