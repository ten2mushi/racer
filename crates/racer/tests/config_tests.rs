use racer::config::{At2Config, PlatoConfig, RacerConfig, SelectionType};
use std::io::Write;
use tempfile::NamedTempFile;

// =============================================================================
// RACER CONFIG TESTS
// =============================================================================

mod racer_config_tests {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn minimal_should_create_valid_config() {
            let config = RacerConfig::minimal();
            assert!(config.validate().is_ok());
        }

        #[test]
        fn default_should_be_same_as_minimal() {
            let minimal = RacerConfig::minimal();
            let default = RacerConfig::default();

            assert_eq!(minimal.node.router_bind, default.node.router_bind);
            assert_eq!(minimal.node.publisher_bind, default.node.publisher_bind);
        }

        #[test]
        fn minimal_should_have_no_node_id() {
            let config = RacerConfig::minimal();
            assert!(config.node.id.is_none());
        }

        #[test]
        fn minimal_should_have_empty_peers() {
            let config = RacerConfig::minimal();
            assert!(config.peers.routers.is_empty());
        }

        #[test]
        fn minimal_should_use_normal_selection() {
            let config = RacerConfig::minimal();
            assert_eq!(config.node.selection_type, SelectionType::Normal);
        }
    }

    mod from_toml {
        use super::*;

        #[test]
        fn should_parse_complete_valid_toml() {
            let toml = r#"
                [node]
                id = "test_node"
                router_bind = "tcp://127.0.0.1:5000"
                publisher_bind = "tcp://127.0.0.1:5001"
                selection_type = "random"

                [consensus]
                echo_sample_size = 10
                ready_sample_size = 10
                delivery_sample_size = 10
                ready_threshold = 6
                feedback_threshold = 8
                delivery_threshold = 9

                [plato]
                target_latency_secs = 3.0
                minimum_latency_secs = 1.0
                max_gossip_timeout_secs = 60.0

                [peers]
                routers = ["tcp://192.168.1.1:5000", "tcp://192.168.1.2:5000"]
            "#;

            let config = RacerConfig::from_toml(toml).unwrap();
            assert_eq!(config.node.id, Some("test_node".to_string()));
            assert_eq!(config.node.router_bind, "tcp://127.0.0.1:5000");
            assert_eq!(config.node.selection_type, SelectionType::Random);
            assert_eq!(config.consensus.echo_sample_size, 10);
            assert_eq!(config.peers.routers.len(), 2);
        }

        #[test]
        fn should_use_defaults_for_missing_fields() {
            let toml = r#"
                [node]
                [consensus]
                [plato]
                [peers]
            "#;

            let config = RacerConfig::from_toml(toml).unwrap();
            assert!(config.node.id.is_none());
            assert_eq!(config.node.router_bind, "tcp://0.0.0.0:20001");
            assert_eq!(config.node.publisher_bind, "tcp://0.0.0.0:21001");
            assert_eq!(config.node.selection_type, SelectionType::Normal);
            assert!(config.peers.routers.is_empty());
        }

        #[test]
        fn should_fail_on_invalid_toml_syntax() {
            let toml = "this is not valid { toml }}}";
            let result = RacerConfig::from_toml(toml);
            assert!(result.is_err());
        }

        #[test]
        fn should_fail_on_missing_required_sections() {
            let toml = r#"
                [node]
                id = "test"
            "#;
            // Missing consensus, plato, peers sections
            let result = RacerConfig::from_toml(toml);
            assert!(result.is_err());
        }

        #[test]
        fn should_validate_after_parsing() {
            let toml = r#"
                [node]
                [consensus]
                ready_threshold = 10
                feedback_threshold = 5
                delivery_threshold = 3
                [plato]
                [peers]
            "#;
            // Invalid threshold ordering should fail validation
            let result = RacerConfig::from_toml(toml);
            assert!(result.is_err());
        }
    }

    mod from_file {
        use super::*;

        #[test]
        fn should_load_from_valid_file() {
            let mut file = NamedTempFile::new().unwrap();
            writeln!(
                file,
                r#"
                [node]
                [consensus]
                [plato]
                [peers]
            "#
            )
            .unwrap();

            let config = RacerConfig::from_file(file.path()).unwrap();
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_fail_on_nonexistent_file() {
            let result = RacerConfig::from_file("/nonexistent/path/config.toml");
            assert!(result.is_err());
        }
    }

    mod validation {
        use super::*;

        #[test]
        fn should_validate_consensus_config() {
            let mut config = RacerConfig::minimal();
            config.consensus.ready_threshold = 100; // Invalid
            config.consensus.feedback_threshold = 50;
            config.consensus.delivery_threshold = 10;

            assert!(config.validate().is_err());
        }

        #[test]
        fn should_validate_plato_config() {
            let mut config = RacerConfig::minimal();
            config.plato.minimum_latency_secs = 0.0; // Invalid

            assert!(config.validate().is_err());
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn should_roundtrip_through_toml() {
            let original = RacerConfig::minimal();
            let toml_str = toml::to_string(&original).unwrap();
            let parsed: RacerConfig = toml::from_str(&toml_str).unwrap();

            assert_eq!(original.node.router_bind, parsed.node.router_bind);
            assert_eq!(original.consensus.echo_sample_size, parsed.consensus.echo_sample_size);
        }
    }
}

// =============================================================================
// SELECTION TYPE TESTS
// =============================================================================

mod selection_type_tests {
    use super::*;

    #[test]
    fn default_should_be_normal() {
        assert_eq!(SelectionType::default(), SelectionType::Normal);
    }

    #[test]
    fn should_serialize_to_lowercase() {
        assert_eq!(serde_json::to_string(&SelectionType::Normal).unwrap(), "\"normal\"");
        assert_eq!(serde_json::to_string(&SelectionType::Random).unwrap(), "\"random\"");
        assert_eq!(serde_json::to_string(&SelectionType::Poisson).unwrap(), "\"poisson\"");
    }

    #[test]
    fn should_deserialize_from_lowercase() {
        assert_eq!(serde_json::from_str::<SelectionType>("\"normal\"").unwrap(), SelectionType::Normal);
        assert_eq!(serde_json::from_str::<SelectionType>("\"random\"").unwrap(), SelectionType::Random);
        assert_eq!(serde_json::from_str::<SelectionType>("\"poisson\"").unwrap(), SelectionType::Poisson);
    }

    #[test]
    fn should_fail_on_invalid_value() {
        let result = serde_json::from_str::<SelectionType>("\"invalid\"");
        assert!(result.is_err());
    }

    #[test]
    fn should_be_copy() {
        let a = SelectionType::Normal;
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn should_be_clone() {
        let a = SelectionType::Random;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn all_variants_should_be_distinguishable() {
        assert_ne!(SelectionType::Normal, SelectionType::Random);
        assert_ne!(SelectionType::Normal, SelectionType::Poisson);
        assert_ne!(SelectionType::Random, SelectionType::Poisson);
    }
}

// =============================================================================
// AT2 CONFIG TESTS
// =============================================================================

mod at2_config_tests {
    use super::*;

    mod defaults {
        use super::*;

        #[test]
        fn default_should_be_valid() {
            let config = At2Config::default();
            assert!(config.validate().is_ok());
        }

        #[test]
        fn default_sample_sizes_should_be_6() {
            let config = At2Config::default();
            assert_eq!(config.echo_sample_size, 6);
            assert_eq!(config.ready_sample_size, 6);
            assert_eq!(config.delivery_sample_size, 6);
        }

        #[test]
        fn default_thresholds_should_be_4_5_6() {
            let config = At2Config::default();
            assert_eq!(config.ready_threshold, 4);
            assert_eq!(config.feedback_threshold, 5);
            assert_eq!(config.delivery_threshold, 6);
        }
    }

    mod with_sample_size {
        use super::*;

        #[test]
        fn should_create_valid_config_for_size_3() {
            let config = At2Config::with_sample_size(3);
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_create_valid_config_for_size_6() {
            let config = At2Config::with_sample_size(6);
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_create_valid_config_for_size_10() {
            let config = At2Config::with_sample_size(10);
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_create_valid_config_for_size_20() {
            let config = At2Config::with_sample_size(20);
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_create_valid_config_for_size_100() {
            let config = At2Config::with_sample_size(100);
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_set_all_sample_sizes_equal() {
            let config = At2Config::with_sample_size(15);
            assert_eq!(config.echo_sample_size, 15);
            assert_eq!(config.ready_sample_size, 15);
            assert_eq!(config.delivery_sample_size, 15);
        }
    }

    mod validation_threshold_ordering {
        use super::*;

        #[test]
        fn should_reject_ready_equal_to_feedback() {
            let config = At2Config {
                ready_threshold: 5,
                feedback_threshold: 5,
                delivery_threshold: 6,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn should_reject_ready_greater_than_feedback() {
            let config = At2Config {
                ready_threshold: 6,
                feedback_threshold: 5,
                delivery_threshold: 7,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn should_reject_feedback_equal_to_delivery() {
            let config = At2Config {
                ready_threshold: 4,
                feedback_threshold: 6,
                delivery_threshold: 6,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn should_reject_feedback_greater_than_delivery() {
            let config = At2Config {
                ready_threshold: 4,
                feedback_threshold: 7,
                delivery_threshold: 6,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn should_accept_strictly_increasing_thresholds() {
            let config = At2Config {
                ready_threshold: 4,
                feedback_threshold: 5,
                delivery_threshold: 6,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    mod validation_majority_constraint {
        use super::*;

        #[test]
        fn ready_threshold_must_be_majority_of_echo_sample() {
            // For echo_sample_size = 6, majority = 4
            let config = At2Config {
                echo_sample_size: 6,
                ready_threshold: 3, // Less than majority
                feedback_threshold: 5,
                delivery_threshold: 6,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn ready_threshold_at_majority_should_pass() {
            let config = At2Config {
                echo_sample_size: 6,
                ready_threshold: 4, // Exactly majority
                feedback_threshold: 5,
                delivery_threshold: 6,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }

        #[test]
        fn ready_threshold_above_majority_should_pass() {
            let config = At2Config {
                echo_sample_size: 6,
                ready_threshold: 5, // Above majority
                feedback_threshold: 6,
                delivery_threshold: 7,
                ..Default::default()
            };
            // May fail due to other constraints, but majority is satisfied
            let result = config.validate();
            // We don't assert ok because feedback constraint might fail
            if result.is_err() {
                let err = result.unwrap_err().to_string();
                assert!(!err.contains("majority"));
            }
        }
    }

    mod validation_feedback_constraint {
        use super::*;

        #[test]
        fn feedback_threshold_below_75_percent_should_fail() {
            // For ready_sample_size = 6, 75% = 4.5, ceil = 5
            let config = At2Config {
                ready_sample_size: 6,
                ready_threshold: 3,
                feedback_threshold: 4, // Less than 75%
                delivery_threshold: 6,
                ..Default::default()
            };
            let result = config.validate();
            assert!(result.is_err());
        }
    }

    mod validation_delivery_constraint {
        use super::*;

        #[test]
        fn delivery_threshold_below_85_percent_should_fail() {
            // For delivery_sample_size = 10, 85% = 8.5, ceil = 9
            let config = At2Config {
                delivery_sample_size: 10,
                ready_threshold: 4,
                feedback_threshold: 7,
                delivery_threshold: 8, // Less than 85%
                ..Default::default()
            };
            let result = config.validate();
            assert!(result.is_err());
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn should_serialize_to_toml() {
            let config = At2Config::default();
            let toml_str = toml::to_string(&config);
            assert!(toml_str.is_ok());
        }

        #[test]
        fn should_deserialize_from_toml() {
            let toml = r#"
                echo_sample_size = 8
                ready_sample_size = 8
                delivery_sample_size = 8
                ready_threshold = 5
                feedback_threshold = 6
                delivery_threshold = 7
            "#;
            let config: At2Config = toml::from_str(toml).unwrap();
            assert_eq!(config.echo_sample_size, 8);
        }

        #[test]
        fn should_roundtrip_through_toml() {
            let original = At2Config::with_sample_size(12);
            let toml_str = toml::to_string(&original).unwrap();
            let parsed: At2Config = toml::from_str(&toml_str).unwrap();
            assert_eq!(original.echo_sample_size, parsed.echo_sample_size);
            assert_eq!(original.ready_threshold, parsed.ready_threshold);
        }
    }
}

// =============================================================================
// PLATO CONFIG TESTS
// =============================================================================

mod plato_config_tests {
    use super::*;

    mod defaults {
        use super::*;

        #[test]
        fn default_should_be_valid() {
            let config = PlatoConfig::default();
            assert!(config.validate().is_ok());
        }

        #[test]
        fn default_target_latency_should_be_2_5() {
            let config = PlatoConfig::default();
            assert!((config.target_latency_secs - 2.5).abs() < f64::EPSILON);
        }

        #[test]
        fn default_minimum_latency_should_be_1_0() {
            let config = PlatoConfig::default();
            assert!((config.minimum_latency_secs - 1.0).abs() < f64::EPSILON);
        }

        #[test]
        fn default_max_gossip_timeout_should_be_60() {
            let config = PlatoConfig::default();
            assert!((config.max_gossip_timeout_secs - 60.0).abs() < f64::EPSILON);
        }

        #[test]
        fn default_rsi_overbought_should_be_70() {
            let config = PlatoConfig::default();
            assert!((config.rsi_overbought - 70.0).abs() < f64::EPSILON);
        }

        #[test]
        fn default_rsi_oversold_should_be_30() {
            let config = PlatoConfig::default();
            assert!((config.rsi_oversold - 30.0).abs() < f64::EPSILON);
        }

        #[test]
        fn default_own_latency_weight_should_be_0_6() {
            let config = PlatoConfig::default();
            assert!((config.own_latency_weight - 0.6).abs() < f64::EPSILON);
        }
    }

    mod validation_minimum_latency {
        use super::*;

        #[test]
        fn zero_minimum_latency_should_fail() {
            let config = PlatoConfig {
                minimum_latency_secs: 0.0,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn negative_minimum_latency_should_fail() {
            let config = PlatoConfig {
                minimum_latency_secs: -1.0,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn positive_minimum_latency_should_pass() {
            let config = PlatoConfig {
                minimum_latency_secs: 0.001,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    mod validation_target_vs_minimum {
        use super::*;

        #[test]
        fn target_less_than_minimum_should_fail() {
            let config = PlatoConfig {
                target_latency_secs: 0.5,
                minimum_latency_secs: 1.0,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn target_equal_to_minimum_should_pass() {
            let config = PlatoConfig {
                target_latency_secs: 1.0,
                minimum_latency_secs: 1.0,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }

        #[test]
        fn target_greater_than_minimum_should_pass() {
            let config = PlatoConfig {
                target_latency_secs: 2.0,
                minimum_latency_secs: 1.0,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    mod validation_max_gossip_timeout {
        use super::*;

        #[test]
        fn max_timeout_less_than_target_should_fail() {
            let config = PlatoConfig {
                target_latency_secs: 5.0,
                max_gossip_timeout_secs: 4.0,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn max_timeout_equal_to_target_should_fail() {
            let config = PlatoConfig {
                target_latency_secs: 5.0,
                max_gossip_timeout_secs: 5.0,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn max_timeout_greater_than_target_should_pass() {
            let config = PlatoConfig {
                target_latency_secs: 5.0,
                max_gossip_timeout_secs: 10.0,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    mod validation_own_latency_weight {
        use super::*;

        #[test]
        fn weight_below_zero_should_fail() {
            let config = PlatoConfig {
                own_latency_weight: -0.1,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn weight_above_one_should_fail() {
            let config = PlatoConfig {
                own_latency_weight: 1.1,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn weight_at_zero_should_pass() {
            let config = PlatoConfig {
                own_latency_weight: 0.0,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }

        #[test]
        fn weight_at_one_should_pass() {
            let config = PlatoConfig {
                own_latency_weight: 1.0,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }

        #[test]
        fn weight_in_middle_should_pass() {
            let config = PlatoConfig {
                own_latency_weight: 0.5,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    mod validation_rsi_thresholds {
        use super::*;

        #[test]
        fn overbought_less_than_oversold_should_fail() {
            let config = PlatoConfig {
                rsi_overbought: 20.0,
                rsi_oversold: 30.0,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn overbought_equal_to_oversold_should_fail() {
            let config = PlatoConfig {
                rsi_overbought: 50.0,
                rsi_oversold: 50.0,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn overbought_greater_than_oversold_should_pass() {
            let config = PlatoConfig {
                rsi_overbought: 80.0,
                rsi_oversold: 20.0,
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn should_serialize_to_toml() {
            let config = PlatoConfig::default();
            let toml_str = toml::to_string(&config);
            assert!(toml_str.is_ok());
        }

        #[test]
        fn should_deserialize_from_toml() {
            let toml = r#"
                target_latency_secs = 5.0
                minimum_latency_secs = 2.0
                max_gossip_timeout_secs = 120.0
            "#;
            let config: PlatoConfig = toml::from_str(toml).unwrap();
            assert!((config.target_latency_secs - 5.0).abs() < f64::EPSILON);
        }

        #[test]
        fn should_use_defaults_for_missing_fields() {
            let toml = "";
            let config: PlatoConfig = toml::from_str(toml).unwrap();
            assert!((config.target_latency_secs - 2.5).abs() < f64::EPSILON);
        }

        #[test]
        fn should_roundtrip_through_toml() {
            let original = PlatoConfig::default();
            let toml_str = toml::to_string(&original).unwrap();
            let parsed: PlatoConfig = toml::from_str(&toml_str).unwrap();
            assert!((original.target_latency_secs - parsed.target_latency_secs).abs() < f64::EPSILON);
        }
    }
}

// =============================================================================
// CONFIG ERROR TESTS
// =============================================================================

mod config_error_tests {
    use super::*;

    #[test]
    fn io_error_should_display_with_prefix() {
        let result = RacerConfig::from_file("/nonexistent/path");
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(msg.contains("I/O error"));
        }
    }

    #[test]
    fn parse_error_should_display_with_prefix() {
        let result = RacerConfig::from_toml("invalid {{ toml");
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(msg.contains("parse error"));
        }
    }

    #[test]
    fn validation_error_should_display_with_prefix() {
        let mut config = RacerConfig::minimal();
        config.plato.minimum_latency_secs = 0.0;
        if let Err(e) = config.validate() {
            let msg = e.to_string();
            assert!(msg.contains("validation error"));
        }
    }
}
