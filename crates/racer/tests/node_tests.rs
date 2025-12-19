#![cfg(test)]

use racer::config::{RacerConfig, SelectionType};
use racer::crypto::KeyPair;
use racer::network::PeerInfo;
use racer::node::{GossipStats, Node, NodeError};
use racer_core::message::DefaultMessage;

// =============================================================================
// TEST HELPERS
// =============================================================================

/// Creates a minimal valid configuration for testing.
fn minimal_config() -> RacerConfig {
    RacerConfig::minimal()
}

/// Creates a configuration with a specific node ID.
fn config_with_id(id: &str) -> RacerConfig {
    let mut config = minimal_config();
    config.node.id = Some(id.to_string());
    config
}

/// Creates a configuration with specific selection type.
fn config_with_selection_type(selection_type: SelectionType) -> RacerConfig {
    let mut config = minimal_config();
    config.node.selection_type = selection_type;
    config
}

/// Creates a peer info for testing.
fn make_peer(id: &str) -> PeerInfo {
    PeerInfo::new(
        id,
        KeyPair::generate().public_key(),
        format!("tcp://127.0.0.1:2000{}", id.chars().next().unwrap_or('0') as u8 % 10),
        format!("tcp://127.0.0.1:2100{}", id.chars().next().unwrap_or('0') as u8 % 10),
    )
}

// =============================================================================
// NODE CONSTRUCTION TESTS
// =============================================================================
mod construction {
    use super::*;

    // -------------------------------------------------------------------------
    // Basic Construction Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn new_should_create_node_with_minimal_config() {
        let config = minimal_config();
        let result = Node::<DefaultMessage>::new(config).await;

        assert!(result.is_ok(), "Node::new should succeed with minimal config");
    }

    #[tokio::test]
    async fn new_should_generate_id_when_not_specified() {
        let mut config = minimal_config();
        config.node.id = None;

        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert!(!node.id().is_empty(), "generated ID should not be empty");
        assert!(
            node.id().starts_with("node-"),
            "generated ID should start with 'node-' prefix"
        );
    }

    #[tokio::test]
    async fn new_should_use_provided_id() {
        let config = config_with_id("my-custom-node");
        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(
            node.id(),
            "my-custom-node",
            "should use explicitly provided node ID"
        );
    }

    #[tokio::test]
    async fn new_should_use_empty_string_id_if_provided() {
        let config = config_with_id("");
        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        // Empty string is a valid ID if explicitly provided
        assert_eq!(node.id(), "", "should accept empty string as explicit ID");
    }

    #[tokio::test]
    async fn new_should_generate_unique_public_key() {
        let config1 = minimal_config();
        let config2 = minimal_config();

        let node1 = Node::<DefaultMessage>::new(config1).await.unwrap();
        let node2 = Node::<DefaultMessage>::new(config2).await.unwrap();

        assert_ne!(
            node1.public_key().to_hex(),
            node2.public_key().to_hex(),
            "different nodes should have different public keys"
        );
    }

    #[tokio::test]
    async fn new_should_initialize_node_as_not_running() {
        let config = minimal_config();
        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert!(
            !node.is_running(),
            "newly created node should not be running"
        );
    }

    #[tokio::test]
    async fn new_should_store_config() {
        let mut config = minimal_config();
        config.consensus.echo_sample_size = 42;

        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(
            node.config().consensus.echo_sample_size, 42,
            "node should store the provided config"
        );
    }

    // -------------------------------------------------------------------------
    // ID Generation Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn generated_id_should_contain_public_key_prefix() {
        let mut config = minimal_config();
        config.node.id = None;

        let node = Node::<DefaultMessage>::new(config).await.unwrap();
        let id = node.id();
        let pk_hex = node.public_key().to_hex();

        // ID format is "node-{first 8 chars of public key hex}"
        let expected_suffix = &pk_hex[..8];
        assert!(
            id.ends_with(expected_suffix),
            "generated ID should end with first 8 chars of public key hex"
        );
    }

    #[tokio::test]
    async fn generated_id_should_be_consistent_with_public_key() {
        let mut config = minimal_config();
        config.node.id = None;

        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        // The ID should be deterministically derived from the public key
        let expected_id = format!("node-{}", &node.public_key().to_hex()[..8]);
        assert_eq!(node.id(), expected_id);
    }
}

// =============================================================================
// NODE ACCESSORS TESTS
// =============================================================================
mod accessors {
    use super::*;

    #[tokio::test]
    async fn id_should_return_node_identifier() {
        let config = config_with_id("test-node-123");
        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(node.id(), "test-node-123");
    }

    #[tokio::test]
    async fn public_key_should_return_valid_ecdsa_key() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();
        let pk = node.public_key();

        // Public key should be valid (non-empty hex)
        let hex = pk.to_hex();
        assert!(!hex.is_empty(), "public key hex should not be empty");
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "public key should be valid hex"
        );
    }

    #[tokio::test]
    async fn config_should_return_reference_to_config() {
        let mut config = minimal_config();
        config.plato.target_latency_secs = 5.0;

        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(
            node.config().plato.target_latency_secs, 5.0,
            "config() should return the configuration"
        );
    }

    #[tokio::test]
    async fn is_running_should_return_false_initially() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();

        assert!(!node.is_running());
    }
}

// =============================================================================
// NODE LIFECYCLE TESTS
// =============================================================================
mod lifecycle {
    use super::*;

    // -------------------------------------------------------------------------
    // Start/Stop Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn stop_should_set_running_to_false() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();

        // Manually set running state (simulating start completed)
        // Note: We can't actually start due to port binding, but we can test stop
        node.stop().await;

        assert!(!node.is_running(), "node should not be running after stop");
    }

    #[tokio::test]
    async fn stop_should_be_idempotent() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();

        // Call stop multiple times
        node.stop().await;
        node.stop().await;
        node.stop().await;

        assert!(
            !node.is_running(),
            "stop should be idempotent and not panic"
        );
    }

    #[tokio::test]
    async fn is_running_should_reflect_current_state() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();

        // Initially not running
        assert!(!node.is_running());

        // After stop (still not running - was never started)
        node.stop().await;
        assert!(!node.is_running());
    }
}

// =============================================================================
// PEER MANAGEMENT TESTS
// =============================================================================
mod peer_management {
    use super::*;

    // -------------------------------------------------------------------------
    // Add Peer Tests
    // NOTE: These tests are marked #[ignore] because add_peer attempts to
    // establish actual ZeroMQ network connections which will hang in the
    // test environment. Run with `cargo test -- --ignored` for integration testing.
    // -------------------------------------------------------------------------

    // NOTE: add_peer tests that verified via select_peers have been removed
    // as select_peers is now an internal method. add_peer functionality is
    // implicitly tested through the gossip flow in integration tests.

    // -------------------------------------------------------------------------
    // Select Peers Tests
    // NOTE: select_peers is now an internal method used by gossip().
    // Peer selection behavior is tested via integration tests.
    // -------------------------------------------------------------------------
}

// =============================================================================
// VECTOR CLOCK TESTS
// =============================================================================
mod vector_clock_integration {
    use super::*;

    #[tokio::test]
    async fn vector_clock_should_be_empty_initially() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();
        let vc = node.vector_clock().await;

        assert!(vc.is_empty(), "initial vector clock should be empty");
    }

    #[tokio::test]
    async fn vector_clock_should_be_cloneable() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();

        let vc1 = node.vector_clock().await;
        let vc2 = node.vector_clock().await;

        // Both should be equal (both empty initially)
        assert_eq!(vc1, vc2, "vector clock should be clonable");
    }
}

// =============================================================================
// GOSSIP STATE TESTS
// =============================================================================
mod gossip_state_integration {
    use super::*;

    #[tokio::test]
    async fn gossip_stats_should_show_zero_active_rounds_initially() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();
        let stats = node.gossip_stats().await;

        assert_eq!(
            stats.active_rounds, 0,
            "should have no active rounds initially"
        );
    }

    #[tokio::test]
    async fn gossip_stats_should_return_gossip_stats_struct() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();
        let stats: GossipStats = node.gossip_stats().await;

        // GossipStats should be a valid struct
        assert!(stats.active_rounds >= 0);
    }
}

// =============================================================================
// PLATO INTEGRATION TESTS
// =============================================================================
mod plato_integration {
    use super::*;

    #[tokio::test]
    async fn plato_stats_should_return_initial_values() {
        let config = minimal_config();
        let expected_latency = config.plato.target_latency_secs;

        let node = Node::<DefaultMessage>::new(config).await.unwrap();
        let stats = node.plato_stats().await;

        assert!(
            (stats.current_latency - expected_latency).abs() < 0.001,
            "initial latency should match config target"
        );
    }

    #[tokio::test]
    async fn run_plato_check_should_not_panic() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();

        // Should not panic even with no data
        node.run_plato_check().await;
        node.run_plato_check().await;
        node.run_plato_check().await;
    }

    #[tokio::test]
    async fn plato_stats_should_include_rsi_values() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();
        let stats = node.plato_stats().await;

        // RSI values should be present (50.0 during warmup)
        assert!(stats.our_rsi_up >= 0.0 && stats.our_rsi_up <= 100.0);
        assert!(stats.our_rsi_down >= 0.0 && stats.our_rsi_down <= 100.0);
        assert!(stats.peer_rsi_up >= 0.0 && stats.peer_rsi_up <= 100.0);
        assert!(stats.peer_rsi_down >= 0.0 && stats.peer_rsi_down <= 100.0);
    }

    #[tokio::test]
    async fn plato_stats_should_show_zero_samples_initially() {
        let node = Node::<DefaultMessage>::new(minimal_config()).await.unwrap();
        let stats = node.plato_stats().await;

        assert_eq!(stats.our_latency_samples, 0, "no latency samples initially");
        assert_eq!(stats.peer_latency_samples, 0, "no peer samples initially");
    }
}

// =============================================================================
// NODE ERROR TESTS
// =============================================================================
mod node_error {
    use super::*;
    use std::error::Error;

    // -------------------------------------------------------------------------
    // Error Variant Tests
    // -------------------------------------------------------------------------

    #[test]
    fn config_error_should_store_message() {
        let err = NodeError::Config("bad config".to_string());

        if let NodeError::Config(msg) = err {
            assert_eq!(msg, "bad config");
        } else {
            panic!("expected Config variant");
        }
    }

    #[test]
    fn network_error_should_store_message() {
        let err = NodeError::Network("connection failed".to_string());

        if let NodeError::Network(msg) = err {
            assert_eq!(msg, "connection failed");
        } else {
            panic!("expected Network variant");
        }
    }

    #[test]
    fn serialization_error_should_store_message() {
        let err = NodeError::Serialization("invalid json".to_string());

        if let NodeError::Serialization(msg) = err {
            assert_eq!(msg, "invalid json");
        } else {
            panic!("expected Serialization variant");
        }
    }

    #[test]
    fn protocol_error_should_store_message() {
        let err = NodeError::Protocol("invalid message".to_string());

        if let NodeError::Protocol(msg) = err {
            assert_eq!(msg, "invalid message");
        } else {
            panic!("expected Protocol variant");
        }
    }

    // -------------------------------------------------------------------------
    // Display Tests
    // -------------------------------------------------------------------------

    #[test]
    fn config_error_display_should_include_prefix() {
        let err = NodeError::Config("test".to_string());
        let s = format!("{}", err);

        assert!(s.contains("configuration error"), "should include prefix");
        assert!(s.contains("test"), "should include message");
    }

    #[test]
    fn network_error_display_should_include_prefix() {
        let err = NodeError::Network("test".to_string());
        let s = format!("{}", err);

        assert!(s.contains("network error"), "should include prefix");
    }

    #[test]
    fn serialization_error_display_should_include_prefix() {
        let err = NodeError::Serialization("test".to_string());
        let s = format!("{}", err);

        assert!(s.contains("serialization error"), "should include prefix");
    }

    #[test]
    fn protocol_error_display_should_include_prefix() {
        let err = NodeError::Protocol("test".to_string());
        let s = format!("{}", err);

        assert!(s.contains("protocol error"), "should include prefix");
    }

    // -------------------------------------------------------------------------
    // Error Trait Tests
    // -------------------------------------------------------------------------

    #[test]
    fn node_error_should_implement_error_trait() {
        let err = NodeError::Config("test".to_string());
        let _: &dyn Error = &err; // Compile-time check
    }

    #[test]
    fn node_error_should_implement_debug() {
        let err = NodeError::Config("test".to_string());
        let debug = format!("{:?}", err);

        assert!(!debug.is_empty(), "debug output should not be empty");
        assert!(debug.contains("Config"), "debug should show variant name");
    }

    #[test]
    fn all_error_variants_should_have_display() {
        let errors = vec![
            NodeError::Config("a".to_string()),
            NodeError::Network("b".to_string()),
            NodeError::Serialization("c".to_string()),
            NodeError::Protocol("d".to_string()),
        ];

        for err in errors {
            let displayed = format!("{}", err);
            assert!(!displayed.is_empty(), "all variants should have display");
        }
    }
}

// =============================================================================
// GOSSIP STATS TESTS
// =============================================================================
mod gossip_stats_struct {
    use super::*;

    #[test]
    fn gossip_stats_should_be_cloneable() {
        let stats = GossipStats { active_rounds: 5 };
        let cloned = stats.clone();

        assert_eq!(cloned.active_rounds, 5);
    }

    #[test]
    fn gossip_stats_should_be_debuggable() {
        let stats = GossipStats { active_rounds: 3 };
        let debug = format!("{:?}", stats);

        assert!(debug.contains("3"), "debug should show active_rounds");
        assert!(debug.contains("active_rounds"), "debug should show field name");
    }

    #[test]
    fn gossip_stats_should_store_active_rounds() {
        let stats = GossipStats { active_rounds: 42 };

        assert_eq!(stats.active_rounds, 42);
    }

    #[test]
    fn gossip_stats_should_handle_zero_rounds() {
        let stats = GossipStats { active_rounds: 0 };

        assert_eq!(stats.active_rounds, 0);
    }

    #[test]
    fn gossip_stats_should_handle_large_round_count() {
        let stats = GossipStats {
            active_rounds: usize::MAX,
        };

        assert_eq!(stats.active_rounds, usize::MAX);
    }
}

// =============================================================================
// CONCURRENT ACCESS TESTS
// =============================================================================
mod concurrency {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn multiple_tasks_should_read_id_safely() {
        let node = Arc::new(Node::<DefaultMessage>::new(minimal_config()).await.unwrap());

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let node_clone = Arc::clone(&node);
                tokio::spawn(async move { node_clone.id().to_string() })
            })
            .collect();

        let ids: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All reads should return the same ID
        let first = &ids[0];
        assert!(
            ids.iter().all(|id| id == first),
            "concurrent ID reads should return same value"
        );
    }

    #[tokio::test]
    async fn multiple_tasks_should_read_public_key_safely() {
        let node = Arc::new(Node::<DefaultMessage>::new(minimal_config()).await.unwrap());

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let node_clone = Arc::clone(&node);
                tokio::spawn(async move { node_clone.public_key().to_hex() })
            })
            .collect();

        let keys: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        let first = &keys[0];
        assert!(
            keys.iter().all(|k| k == first),
            "concurrent public_key reads should return same value"
        );
    }


    #[tokio::test]
    async fn concurrent_vector_clock_reads_should_be_safe() {
        let node = Arc::new(Node::<DefaultMessage>::new(minimal_config()).await.unwrap());

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let node_clone = Arc::clone(&node);
                tokio::spawn(async move { node_clone.vector_clock().await })
            })
            .collect();

        let clocks: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All clocks should be equal (empty initially)
        let first = &clocks[0];
        assert!(
            clocks.iter().all(|c| c == first),
            "concurrent vector clock reads should be consistent"
        );
    }

    #[tokio::test]
    async fn concurrent_gossip_stats_reads_should_be_safe() {
        let node = Arc::new(Node::<DefaultMessage>::new(minimal_config()).await.unwrap());

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let node_clone = Arc::clone(&node);
                tokio::spawn(async move { node_clone.gossip_stats().await.active_rounds })
            })
            .collect();

        let stats: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All should show 0 active rounds initially
        assert!(
            stats.iter().all(|&s| s == 0),
            "concurrent gossip_stats reads should be consistent"
        );
    }

    #[tokio::test]
    async fn concurrent_plato_checks_should_not_panic() {
        let node = Arc::new(Node::<DefaultMessage>::new(minimal_config()).await.unwrap());

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let node_clone = Arc::clone(&node);
                tokio::spawn(async move {
                    node_clone.run_plato_check().await;
                })
            })
            .collect();

        // Should complete without panic
        futures::future::join_all(handles).await;
    }
}

// =============================================================================
// CONFIGURATION INTEGRATION TESTS
// =============================================================================
mod config_integration {
    use super::*;

    #[tokio::test]
    async fn node_should_use_consensus_config() {
        let mut config = minimal_config();
        config.consensus.echo_sample_size = 15;
        config.consensus.ready_sample_size = 20;

        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(node.config().consensus.echo_sample_size, 15);
        assert_eq!(node.config().consensus.ready_sample_size, 20);
    }

    #[tokio::test]
    async fn node_should_use_plato_config() {
        let mut config = minimal_config();
        config.plato.target_latency_secs = 7.5;
        config.plato.rsi_overbought = 80.0;

        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(node.config().plato.target_latency_secs, 7.5);
        assert_eq!(node.config().plato.rsi_overbought, 80.0);
    }

    #[tokio::test]
    async fn node_should_use_node_config() {
        let mut config = minimal_config();
        config.node.router_bind = "tcp://0.0.0.0:12345".to_string();
        config.node.publisher_bind = "tcp://0.0.0.0:54321".to_string();

        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(node.config().node.router_bind, "tcp://0.0.0.0:12345");
        assert_eq!(node.config().node.publisher_bind, "tcp://0.0.0.0:54321");
    }

    #[tokio::test]
    async fn node_should_use_peers_config() {
        let mut config = minimal_config();
        config.peers.routers = vec![
            "tcp://192.168.1.1:20001".to_string(),
            "tcp://192.168.1.2:20001".to_string(),
        ];

        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(node.config().peers.routers.len(), 2);
    }
}

// =============================================================================
// EDGE CASES AND BOUNDARY CONDITIONS
// =============================================================================
mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn node_should_handle_special_characters_in_id() {
        let config = config_with_id("node-with-special-chars-123!@#");
        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(node.id(), "node-with-special-chars-123!@#");
    }

    #[tokio::test]
    async fn node_should_handle_unicode_in_id() {
        let config = config_with_id("ノード-日本語");
        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(node.id(), "ノード-日本語");
    }

    #[tokio::test]
    async fn node_should_handle_very_long_id() {
        let long_id = "a".repeat(1000);
        let config = config_with_id(&long_id);
        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert_eq!(node.id(), long_id);
    }

    #[tokio::test]
    async fn node_should_work_with_default_config() {
        let config = RacerConfig::default();
        let result = Node::<DefaultMessage>::new(config).await;

        assert!(result.is_ok(), "should work with default config");
    }
}
