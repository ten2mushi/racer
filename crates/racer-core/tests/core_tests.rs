#![cfg(test)]

mod message_tests {
    use racer_core::message::{DefaultMessage, Message};
    use serde::{Deserialize, Serialize};

    mod construction {
        use super::*;

        #[test]
        fn new_should_return_message_with_current_timestamp() {
            let before = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            let msg = DefaultMessage::new();
            
            let after = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            assert!(msg.timestamp >= before, "timestamp should be >= time before creation");
            assert!(msg.timestamp <= after, "timestamp should be <= time after creation");
        }

        #[test]
        fn new_should_set_padding_to_zero() {
            let msg = DefaultMessage::new();
            assert_eq!(msg.padding, 0, "new() should initialize padding to 0");
        }

        #[test]
        fn with_padding_should_set_specified_padding_value() {
            let msg = DefaultMessage::with_padding(12345);
            assert_eq!(msg.padding, 12345, "with_padding should set the exact padding value");
        }

        #[test]
        fn with_padding_should_still_set_current_timestamp() {
            let before = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            let msg = DefaultMessage::with_padding(100);
            
            assert!(msg.timestamp >= before, "with_padding should also capture current timestamp");
        }

        #[test]
        fn with_padding_should_handle_zero_padding() {
            let msg = DefaultMessage::with_padding(0);
            assert_eq!(msg.padding, 0, "with_padding(0) should work correctly");
        }

        #[test]
        fn with_padding_should_handle_max_u64_padding() {
            let msg = DefaultMessage::with_padding(u64::MAX);
            assert_eq!(msg.padding, u64::MAX, "with_padding should handle max u64 value");
        }

        #[test]
        fn default_should_return_zeroed_message() {
            let msg = DefaultMessage::default();
            assert_eq!(msg.timestamp, 0, "Default::default() should set timestamp to 0");
            assert_eq!(msg.padding, 0, "Default::default() should set padding to 0");
        }
    }

    mod message_trait {
        use super::*;

        #[test]
        fn id_should_return_timestamp() {
            let msg = DefaultMessage { timestamp: 42, padding: 100 };
            assert_eq!(msg.id(), 42, "id() should return the timestamp field");
        }

        #[test]
        fn id_should_return_zero_for_default() {
            let msg = DefaultMessage::default();
            assert_eq!(msg.id(), 0, "id() should return 0 for default message");
        }

        #[test]
        fn id_should_return_max_for_max_timestamp() {
            let msg = DefaultMessage { timestamp: u64::MAX, padding: 0 };
            assert_eq!(msg.id(), u64::MAX, "id() should handle max u64 timestamp");
        }

        #[test]
        fn merkle_bytes_should_return_valid_json() {
            let msg = DefaultMessage { timestamp: 123, padding: 456 };
            let bytes = msg.merkle_bytes();
            
            // Must be valid JSON
            let result: serde_json::Result<serde_json::Value> = serde_json::from_slice(&bytes);
            assert!(result.is_ok(), "merkle_bytes should return valid JSON");
        }

        #[test]
        fn merkle_bytes_should_be_reversible() {
            let original = DefaultMessage { timestamp: 999, padding: 888 };
            let bytes = original.merkle_bytes();
            let parsed: DefaultMessage = serde_json::from_slice(&bytes).unwrap();
            
            assert_eq!(parsed.timestamp, original.timestamp, "timestamp should survive roundtrip");
            assert_eq!(parsed.padding, original.padding, "padding should survive roundtrip");
        }

        #[test]
        fn merkle_bytes_should_produce_different_output_for_different_messages() {
            let msg1 = DefaultMessage { timestamp: 1, padding: 0 };
            let msg2 = DefaultMessage { timestamp: 2, padding: 0 };
            
            assert_ne!(msg1.merkle_bytes(), msg2.merkle_bytes(), 
                "different messages should produce different merkle bytes");
        }

        #[test]
        fn merkle_bytes_should_produce_same_output_for_same_messages() {
            let msg1 = DefaultMessage { timestamp: 100, padding: 50 };
            let msg2 = DefaultMessage { timestamp: 100, padding: 50 };
            
            assert_eq!(msg1.merkle_bytes(), msg2.merkle_bytes(),
                "identical messages should produce identical merkle bytes");
        }

        #[test]
        fn validate_should_always_succeed_for_default_message() {
            let msg = DefaultMessage::new();
            assert!(msg.validate().is_ok(), "DefaultMessage.validate() should always succeed");
        }

        #[test]
        fn validate_should_succeed_for_default_constructed_message() {
            let msg = DefaultMessage::default();
            assert!(msg.validate().is_ok(), "Default-constructed message should validate successfully");
        }
    }

    mod derived_traits {
        use super::*;

        #[test]
        fn clone_should_produce_equal_message() {
            let original = DefaultMessage { timestamp: 42, padding: 24 };
            let cloned = original.clone();
            
            assert_eq!(cloned.timestamp, original.timestamp);
            assert_eq!(cloned.padding, original.padding);
        }

        #[test]
        fn debug_should_format_message_contents() {
            let msg = DefaultMessage { timestamp: 123, padding: 456 };
            let debug_str = format!("{:?}", msg);
            
            assert!(debug_str.contains("123"), "debug output should contain timestamp");
            assert!(debug_str.contains("456"), "debug output should contain padding");
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn should_serialize_to_json() {
            let msg = DefaultMessage { timestamp: 100, padding: 200 };
            let json = serde_json::to_string(&msg).unwrap();
            
            assert!(json.contains("\"timestamp\":100"), "JSON should contain timestamp field");
            assert!(json.contains("\"padding\":200"), "JSON should contain padding field");
        }

        #[test]
        fn should_deserialize_from_json() {
            let json = r#"{"timestamp":999,"padding":888}"#;
            let msg: DefaultMessage = serde_json::from_str(json).unwrap();
            
            assert_eq!(msg.timestamp, 999);
            assert_eq!(msg.padding, 888);
        }

        #[test]
        fn should_handle_missing_fields_with_default() {
            let json = r#"{}"#;
            let result: serde_json::Result<DefaultMessage> = serde_json::from_str(json);
            assert!(result.is_err(), "missing required fields should fail deserialization");
        }

        #[test]
        fn should_handle_extra_fields_gracefully() {
            let json = r#"{"timestamp":1,"padding":2,"extra":"ignored"}"#;
            let msg: DefaultMessage = serde_json::from_str(json).unwrap();
            
            assert_eq!(msg.timestamp, 1, "should deserialize even with extra fields");
            assert_eq!(msg.padding, 2);
        }

        #[test]
        fn roundtrip_should_preserve_all_values() {
            let original = DefaultMessage { timestamp: u64::MAX, padding: u64::MAX - 1 };
            let json = serde_json::to_string(&original).unwrap();
            let restored: DefaultMessage = serde_json::from_str(&json).unwrap();
            
            assert_eq!(restored.timestamp, original.timestamp);
            assert_eq!(restored.padding, original.padding);
        }
    }

    mod custom_message {
        use super::*;

        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct CustomMessage {
            id: u64,
            name: String,
            values: Vec<f64>,
        }

        impl Message for CustomMessage {
            fn id(&self) -> u64 {
                self.id
            }
        }

        #[test]
        fn custom_message_should_implement_message_trait() {
            let msg = CustomMessage {
                id: 42,
                name: "test".into(),
                values: vec![1.0, 2.0, 3.0],
            };
            
            assert_eq!(msg.id(), 42, "custom id() implementation should work");
        }

        #[test]
        fn custom_message_should_use_default_merkle_bytes() {
            let msg = CustomMessage {
                id: 1,
                name: "hello".into(),
                values: vec![],
            };
            
            let bytes = msg.merkle_bytes();
            let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            
            assert_eq!(json["name"], "hello", "default merkle_bytes should serialize all fields");
        }

        #[test]
        fn custom_message_should_use_default_validate() {
            let msg = CustomMessage {
                id: 1,
                name: String::new(),
                values: vec![],
            };
            
            assert!(msg.validate().is_ok(), "default validate() should return Ok");
        }
    }
}

mod validation_tests {
    use racer_core::validation::{FieldValidator, ValidationError, ValidationKind, ValidationResult};

    mod validation_error_construction {
        use super::*;

        #[test]
        fn new_should_store_all_fields() {
            let err = ValidationError::new("field_name", "error message", ValidationKind::Required);
            
            assert_eq!(err.field, "field_name");
            assert_eq!(err.message, "error message");
            assert_eq!(err.kind, ValidationKind::Required);
        }

        #[test]
        fn required_should_create_required_error() {
            let err = ValidationError::required("username");
            
            assert_eq!(err.field, "username");
            assert_eq!(err.kind, ValidationKind::Required);
            assert!(err.message.contains("username"), "message should mention field name");
            assert!(err.message.contains("required"), "message should mention 'required'");
        }

        #[test]
        fn min_value_should_include_bounds_in_message() {
            let err = ValidationError::min_value("age", 18.0, 15.0);
            
            assert_eq!(err.field, "age");
            assert!(matches!(err.kind, ValidationKind::MinValue { min: 18.0, actual: 15.0 }));
            assert!(err.message.contains("18"), "message should contain min value");
            assert!(err.message.contains("15"), "message should contain actual value");
        }

        #[test]
        fn max_value_should_include_bounds_in_message() {
            let err = ValidationError::max_value("score", 100.0, 105.0);
            
            assert_eq!(err.field, "score");
            assert!(matches!(err.kind, ValidationKind::MaxValue { max: 100.0, actual: 105.0 }));
            assert!(err.message.contains("100"), "message should contain max value");
            assert!(err.message.contains("105"), "message should contain actual value");
        }

        #[test]
        fn min_length_should_include_length_in_message() {
            let err = ValidationError::min_length("password", 8, 5);
            
            assert_eq!(err.field, "password");
            assert!(matches!(err.kind, ValidationKind::MinLength { min: 8, actual: 5 }));
            assert!(err.message.contains("8"), "message should contain min length");
            assert!(err.message.contains("5"), "message should contain actual length");
        }

        #[test]
        fn max_length_should_include_length_in_message() {
            let err = ValidationError::max_length("bio", 500, 750);
            
            assert_eq!(err.field, "bio");
            assert!(matches!(err.kind, ValidationKind::MaxLength { max: 500, actual: 750 }));
            assert!(err.message.contains("500"), "message should contain max length");
            assert!(err.message.contains("750"), "message should contain actual length");
        }
    }

    mod validation_error_display {
        use super::*;

        #[test]
        fn display_should_return_message() {
            let err = ValidationError::new("field", "custom error message", ValidationKind::Required);
            let displayed = format!("{}", err);
            
            assert_eq!(displayed, "custom error message");
        }

        #[test]
        fn display_should_work_for_all_error_types() {
            let errors = vec![
                ValidationError::required("f"),
                ValidationError::min_value("f", 0.0, -1.0),
                ValidationError::max_value("f", 10.0, 11.0),
                ValidationError::min_length("f", 5, 3),
                ValidationError::max_length("f", 10, 15),
            ];
            
            for err in errors {
                let displayed = format!("{}", err);
                assert!(!displayed.is_empty(), "all error types should have non-empty display");
            }
        }
    }

    mod validation_kind {
        use super::*;

        #[test]
        fn required_should_equal_required() {
            assert_eq!(ValidationKind::Required, ValidationKind::Required);
        }

        #[test]
        fn min_value_equality_should_compare_all_fields() {
            let k1 = ValidationKind::MinValue { min: 1.0, actual: 0.5 };
            let k2 = ValidationKind::MinValue { min: 1.0, actual: 0.5 };
            let k3 = ValidationKind::MinValue { min: 2.0, actual: 0.5 };
            
            assert_eq!(k1, k2, "same min and actual should be equal");
            assert_ne!(k1, k3, "different min should not be equal");
        }

        #[test]
        fn different_kinds_should_not_be_equal() {
            let required = ValidationKind::Required;
            let min_val = ValidationKind::MinValue { min: 0.0, actual: 0.0 };
            
            assert_ne!(required, min_val, "different kinds should not be equal");
        }

        #[test]
        fn clone_should_produce_equal_kind() {
            let original = ValidationKind::MaxLength { max: 100, actual: 150 };
            let cloned = original.clone();
            
            assert_eq!(original, cloned, "cloned ValidationKind should be equal");
        }

        #[test]
        fn debug_should_format_all_variants() {
            let variants = vec![
                ValidationKind::Required,
                ValidationKind::MinValue { min: 1.0, actual: 0.0 },
                ValidationKind::MaxValue { max: 10.0, actual: 11.0 },
                ValidationKind::MinLength { min: 5, actual: 3 },
                ValidationKind::MaxLength { max: 10, actual: 15 },
            ];
            
            for v in variants {
                let debug = format!("{:?}", v);
                assert!(!debug.is_empty(), "all variants should have debug representation");
            }
        }
    }

    mod field_validator {
        use super::*;
        use std::collections::HashMap;

        #[test]
        fn empty_string_should_be_empty() {
            assert!(String::new().is_empty());
        }

        #[test]
        fn non_empty_string_should_not_be_empty() {
            assert!(!String::from("hello").is_empty());
        }

        #[test]
        fn whitespace_only_string_should_not_be_empty() {
            // Note: is_empty() checks length, not content
            assert!(!String::from("   ").is_empty(), 
                "whitespace-only string is not empty (has length)");
        }

        #[test]
        fn empty_vec_should_be_empty() {
            assert!(Vec::<i32>::new().is_empty());
        }

        #[test]
        fn non_empty_vec_should_not_be_empty() {
            assert!(!vec![1, 2, 3].is_empty());
        }

        #[test]
        fn vec_with_single_element_should_not_be_empty() {
            assert!(!vec![0].is_empty());
        }

        #[test]
        fn empty_hashmap_should_be_empty() {
            assert!(HashMap::<String, i32>::new().is_empty());
        }

        #[test]
        fn non_empty_hashmap_should_not_be_empty() {
            let mut map = HashMap::new();
            map.insert("key", "value");
            assert!(!map.is_empty());
        }

        #[test]
        fn u8_should_never_be_empty() {
            assert!(!0u8.is_empty());
            assert!(!u8::MAX.is_empty());
        }

        #[test]
        fn u16_should_never_be_empty() {
            assert!(!0u16.is_empty());
            assert!(!u16::MAX.is_empty());
        }

        #[test]
        fn u32_should_never_be_empty() {
            assert!(!0u32.is_empty());
            assert!(!u32::MAX.is_empty());
        }

        #[test]
        fn u64_should_never_be_empty() {
            assert!(!0u64.is_empty());
            assert!(!u64::MAX.is_empty());
        }

        #[test]
        fn i8_should_never_be_empty() {
            assert!(!0i8.is_empty());
            assert!(!i8::MIN.is_empty());
            assert!(!i8::MAX.is_empty());
        }

        #[test]
        fn i16_should_never_be_empty() {
            assert!(!0i16.is_empty());
            assert!(!i16::MIN.is_empty());
        }

        #[test]
        fn i32_should_never_be_empty() {
            assert!(!0i32.is_empty());
            assert!(!(-42i32).is_empty());
        }

        #[test]
        fn i64_should_never_be_empty() {
            assert!(!0i64.is_empty());
            assert!(!i64::MIN.is_empty());
        }

        #[test]
        fn f32_should_never_be_empty() {
            assert!(!0.0f32.is_empty());
            assert!(!f32::NAN.is_empty());
            assert!(!f32::INFINITY.is_empty());
            assert!(!f32::NEG_INFINITY.is_empty());
        }

        #[test]
        fn f64_should_never_be_empty() {
            assert!(!0.0f64.is_empty());
            assert!(!f64::NAN.is_empty());
            assert!(!f64::INFINITY.is_empty());
        }

        #[test]
        fn bool_should_never_be_empty() {
            assert!(!true.is_empty());
            assert!(!false.is_empty());
        }
    }

    mod validation_result {
        use super::*;

        #[test]
        fn ok_result_should_be_ok() {
            let result: ValidationResult = Ok(());
            assert!(result.is_ok());
        }

        #[test]
        fn err_result_should_contain_validation_error() {
            let result: ValidationResult = Err(ValidationError::required("test"));
            assert!(result.is_err());
            
            let err = result.unwrap_err();
            assert_eq!(err.field, "test");
        }
    }
}

mod error_tests {
    use racer_core::error::RacerError;
    use racer_core::validation::ValidationError;
    use std::error::Error;

    mod construction {
        use super::*;

        #[test]
        fn config_should_store_message() {
            let err = RacerError::config("invalid config");
            if let RacerError::Config(msg) = err {
                assert_eq!(msg, "invalid config");
            } else {
                panic!("expected Config variant");
            }
        }

        #[test]
        fn crypto_should_store_message() {
            let err = RacerError::crypto("signature failed");
            if let RacerError::Crypto(msg) = err {
                assert_eq!(msg, "signature failed");
            } else {
                panic!("expected Crypto variant");
            }
        }

        #[test]
        fn network_should_store_message() {
            let err = RacerError::network("connection refused");
            if let RacerError::Network(msg) = err {
                assert_eq!(msg, "connection refused");
            } else {
                panic!("expected Network variant");
            }
        }

        #[test]
        fn protocol_should_store_message() {
            let err = RacerError::protocol("invalid message format");
            if let RacerError::Protocol(msg) = err {
                assert_eq!(msg, "invalid message format");
            } else {
                panic!("expected Protocol variant");
            }
        }

        #[test]
        fn timeout_should_store_message() {
            let err = RacerError::timeout("operation timed out after 30s");
            if let RacerError::Timeout(msg) = err {
                assert_eq!(msg, "operation timed out after 30s");
            } else {
                panic!("expected Timeout variant");
            }
        }

        #[test]
        fn constructors_should_accept_string_slice() {
            let _ = RacerError::config("string slice");
            let _ = RacerError::crypto("string slice");
            let _ = RacerError::network("string slice");
            let _ = RacerError::protocol("string slice");
            let _ = RacerError::timeout("string slice");
        }

        #[test]
        fn constructors_should_accept_owned_string() {
            let _ = RacerError::config(String::from("owned string"));
            let _ = RacerError::crypto(String::from("owned string"));
            let _ = RacerError::network(String::from("owned string"));
            let _ = RacerError::protocol(String::from("owned string"));
            let _ = RacerError::timeout(String::from("owned string"));
        }
    }

    mod display {
        use super::*;

        #[test]
        fn config_display_should_include_prefix() {
            let err = RacerError::Config("bad config".into());
            let s = format!("{}", err);
            assert!(s.contains("configuration error"), "should include error type prefix");
            assert!(s.contains("bad config"), "should include message");
        }

        #[test]
        fn crypto_display_should_include_prefix() {
            let err = RacerError::Crypto("key error".into());
            let s = format!("{}", err);
            assert!(s.contains("crypto error"), "should include error type prefix");
        }

        #[test]
        fn network_display_should_include_prefix() {
            let err = RacerError::Network("disconnected".into());
            let s = format!("{}", err);
            assert!(s.contains("network error"), "should include error type prefix");
        }

        #[test]
        fn protocol_display_should_include_prefix() {
            let err = RacerError::Protocol("invalid".into());
            let s = format!("{}", err);
            assert!(s.contains("protocol error"), "should include error type prefix");
        }

        #[test]
        fn timeout_display_should_include_prefix() {
            let err = RacerError::Timeout("30s".into());
            let s = format!("{}", err);
            assert!(s.contains("timed out"), "should include timeout message");
        }
    }

    mod conversions {
        use super::*;

        #[test]
        fn validation_error_should_convert_to_racer_error() {
            let validation_err = ValidationError::required("field");
            let racer_err: RacerError = validation_err.into();
            
            assert!(matches!(racer_err, RacerError::Validation(_)));
        }

        #[test]
        fn io_error_should_convert_to_racer_error() {
            let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
            let racer_err: RacerError = io_err.into();
            
            assert!(matches!(racer_err, RacerError::Io(_)));
        }

        #[test]
        fn serde_json_error_should_convert_to_racer_error() {
            let json_err = serde_json::from_str::<serde_json::Value>("not valid json").unwrap_err();
            let racer_err: RacerError = json_err.into();
            
            assert!(matches!(racer_err, RacerError::Serialization(_)));
        }
    }

    mod std_error {
        use super::*;

        #[test]
        fn should_implement_error_trait() {
            let err = RacerError::config("test");
            let _: &dyn Error = &err; // Compile-time check
        }

        #[test]
        fn source_should_return_none_for_string_variants() {
            let err = RacerError::config("test");
            assert!(err.source().is_none(), "Config variant has no source");
        }

        #[test]
        fn source_should_return_some_for_nested_errors() {
            let io_err = std::io::Error::new(std::io::ErrorKind::Other, "inner");
            let racer_err: RacerError = io_err.into();
            
            assert!(racer_err.source().is_some(), "Io variant should have source");
        }

        #[test]
        fn debug_should_be_implemented() {
            let err = RacerError::config("test");
            let debug = format!("{:?}", err);
            assert!(!debug.is_empty(), "debug output should not be empty");
        }
    }

    mod result_alias {
        use super::*;
        use racer_core::error::Result;

        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(RacerError::config("error"))
        }

        #[test]
        fn result_ok_should_unwrap_to_value() {
            assert_eq!(returns_ok().unwrap(), 42);
        }

        #[test]
        fn result_err_should_contain_racer_error() {
            let result = returns_err();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), RacerError::Config(_)));
        }
    }
}

mod integration_tests {
    use racer_core::Message;
    use racer_core::message::DefaultMessage;
    use racer_core::validation::ValidationError;
    use racer_core::error::RacerError;

    #[test]
    fn message_validation_failure_should_convert_to_racer_error() {
        let validation_err = ValidationError::required("timestamp");
        let racer_err: RacerError = validation_err.into();
        
        let display = format!("{}", racer_err);
        assert!(display.contains("validation"), "RacerError should wrap validation errors");
    }

    #[test]
    fn default_message_full_workflow() {
        let msg = DefaultMessage::with_padding(100);
        
        assert!(msg.id() > 0, "ID should be positive timestamp");
        
        assert!(msg.validate().is_ok(), "validation should pass");
        
        let bytes = msg.merkle_bytes();
        assert!(!bytes.is_empty(), "merkle bytes should not be empty");
        
        let restored: DefaultMessage = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored.padding, msg.padding, "padding should survive roundtrip");
    }

    #[test]
    fn cloned_message_should_behave_identically() {
        let original = DefaultMessage::with_padding(500);
        let cloned = original.clone();
        
        assert_eq!(original.id(), cloned.id());
        assert_eq!(original.merkle_bytes(), cloned.merkle_bytes());
        assert_eq!(original.validate().is_ok(), cloned.validate().is_ok());
    }
}
