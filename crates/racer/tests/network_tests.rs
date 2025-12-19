use racer::crypto::KeyPair;
use racer::network::{NetworkError, PeerInfo, PeerRegistry, RacerNetwork};

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Creates a test peer with deterministic addresses based on ID.
fn make_peer(id: &str) -> PeerInfo {
    PeerInfo::new(
        id,
        KeyPair::generate().public_key(),
        format!("tcp://127.0.0.1:200{}", id),
        format!("tcp://127.0.0.1:210{}", id),
    )
}

/// Creates a peer with specific addresses for testing.
fn make_peer_with_addresses(id: &str, router: &str, publisher: &str) -> PeerInfo {
    PeerInfo::new(id, KeyPair::generate().public_key(), router, publisher)
}

// =============================================================================
// PEER INFO CONSTRUCTION TESTS
// =============================================================================

mod peer_info_construction {
    use super::*;

    #[test]
    fn new_should_store_id() {
        let peer = make_peer("node1");
        assert_eq!(peer.id, "node1");
    }

    #[test]
    fn new_should_accept_string_slice_for_id() {
        let id: &str = "test_id";
        let peer = make_peer(id);
        assert_eq!(peer.id, "test_id");
    }

    #[test]
    fn new_should_accept_owned_string_for_id() {
        let id = String::from("owned_id");
        let peer = PeerInfo::new(
            id,
            KeyPair::generate().public_key(),
            "tcp://127.0.0.1:5000",
            "tcp://127.0.0.1:5001",
        );
        assert_eq!(peer.id, "owned_id");
    }

    #[test]
    fn new_should_store_router_address() {
        let peer = make_peer_with_addresses("n1", "tcp://10.0.0.1:5555", "tcp://10.0.0.1:5556");
        assert_eq!(peer.router_address, "tcp://10.0.0.1:5555");
    }

    #[test]
    fn new_should_store_publisher_address() {
        let peer = make_peer_with_addresses("n1", "tcp://10.0.0.1:5555", "tcp://10.0.0.1:5556");
        assert_eq!(peer.publisher_address, "tcp://10.0.0.1:5556");
    }

    #[test]
    fn new_should_initialize_reported_latency_to_zero() {
        let peer = make_peer("n1");
        assert!((peer.reported_latency - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn new_should_initialize_last_seen_to_none() {
        let peer = make_peer("n1");
        assert!(peer.last_seen.is_none());
    }

    #[test]
    fn new_should_store_ecdsa_public_key() {
        let kp = KeyPair::generate();
        let pk = kp.public_key();
        let peer = PeerInfo::new("n1", pk.clone(), "tcp://a:1", "tcp://a:2");
        assert_eq!(peer.ecdsa_public, pk);
    }
}

// =============================================================================
// PEER INFO TOUCH TESTS
// =============================================================================

mod peer_info_touch {
    use super::*;
    use std::time::Instant;

    #[test]
    fn touch_should_set_last_seen_to_some() {
        let mut peer = make_peer("n1");
        assert!(peer.last_seen.is_none());
        peer.touch();
        assert!(peer.last_seen.is_some());
    }

    #[test]
    fn touch_should_update_last_seen_timestamp() {
        let mut peer = make_peer("n1");
        let before = Instant::now();
        peer.touch();
        let after = Instant::now();

        let last_seen = peer.last_seen.unwrap();
        assert!(last_seen >= before);
        assert!(last_seen <= after);
    }

    #[test]
    fn touch_called_twice_should_update_to_later_time() {
        let mut peer = make_peer("n1");
        peer.touch();
        let first = peer.last_seen.unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1));

        peer.touch();
        let second = peer.last_seen.unwrap();

        assert!(second > first);
    }
}

// =============================================================================
// PEER INFO SERIALIZATION TESTS
// =============================================================================

mod peer_info_serialization {
    use super::*;

    #[test]
    fn should_serialize_to_json() {
        let peer = make_peer("test_node");
        let json = serde_json::to_string(&peer);
        assert!(json.is_ok());
    }

    #[test]
    fn should_include_id_in_json() {
        let peer = make_peer("my_node");
        let json = serde_json::to_string(&peer).unwrap();
        assert!(json.contains("\"id\":\"my_node\""));
    }

    #[test]
    fn should_include_router_address_in_json() {
        let peer = make_peer_with_addresses("n1", "tcp://host:1234", "tcp://host:1235");
        let json = serde_json::to_string(&peer).unwrap();
        assert!(json.contains("tcp://host:1234"));
    }

    #[test]
    fn should_include_publisher_address_in_json() {
        let peer = make_peer_with_addresses("n1", "tcp://host:1234", "tcp://host:1235");
        let json = serde_json::to_string(&peer).unwrap();
        assert!(json.contains("tcp://host:1235"));
    }

    #[test]
    fn reported_latency_should_be_skipped_in_serialization() {
        let mut peer = make_peer("n1");
        peer.reported_latency = 123.456;
        let json = serde_json::to_string(&peer).unwrap();
        assert!(!json.contains("reported_latency"));
        assert!(!json.contains("123.456"));
    }

    #[test]
    fn last_seen_should_be_skipped_in_serialization() {
        let mut peer = make_peer("n1");
        peer.touch();
        let json = serde_json::to_string(&peer).unwrap();
        assert!(!json.contains("last_seen"));
    }

    #[test]
    fn should_deserialize_from_json() {
        let peer = make_peer("test");
        let json = serde_json::to_string(&peer).unwrap();
        let restored: PeerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, peer.id);
        assert_eq!(restored.router_address, peer.router_address);
        assert_eq!(restored.publisher_address, peer.publisher_address);
    }

    #[test]
    fn deserialized_peer_should_have_default_latency() {
        let peer = make_peer("test");
        let json = serde_json::to_string(&peer).unwrap();
        let restored: PeerInfo = serde_json::from_str(&json).unwrap();
        assert!((restored.reported_latency - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn deserialized_peer_should_have_none_last_seen() {
        let mut peer = make_peer("test");
        peer.touch(); // Set last_seen before serialization
        let json = serde_json::to_string(&peer).unwrap();
        let restored: PeerInfo = serde_json::from_str(&json).unwrap();
        assert!(restored.last_seen.is_none());
    }
}

// =============================================================================
// PEER REGISTRY CONSTRUCTION TESTS
// =============================================================================

mod peer_registry_construction {
    use super::*;

    #[test]
    fn new_should_create_empty_registry() {
        let registry = PeerRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn new_should_have_zero_length() {
        let registry = PeerRegistry::new();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn new_should_have_no_self_id() {
        let registry = PeerRegistry::new();
        assert!(registry.self_id().is_none());
    }

    #[test]
    fn default_should_create_empty_registry() {
        let registry = PeerRegistry::default();
        assert!(registry.is_empty());
    }
}

// =============================================================================
// PEER REGISTRY SELF ID TESTS
// =============================================================================

mod peer_registry_self_id {
    use super::*;

    #[test]
    fn set_self_id_should_store_id() {
        let mut registry = PeerRegistry::new();
        registry.set_self_id("my_node");
        assert_eq!(registry.self_id(), Some("my_node"));
    }

    #[test]
    fn set_self_id_should_accept_string_slice() {
        let mut registry = PeerRegistry::new();
        let id: &str = "slice_id";
        registry.set_self_id(id);
        assert_eq!(registry.self_id(), Some("slice_id"));
    }

    #[test]
    fn set_self_id_should_accept_owned_string() {
        let mut registry = PeerRegistry::new();
        registry.set_self_id(String::from("owned_id"));
        assert_eq!(registry.self_id(), Some("owned_id"));
    }

    #[test]
    fn set_self_id_called_twice_should_overwrite() {
        let mut registry = PeerRegistry::new();
        registry.set_self_id("first");
        registry.set_self_id("second");
        assert_eq!(registry.self_id(), Some("second"));
    }

    #[test]
    fn self_id_should_return_none_before_set() {
        let registry = PeerRegistry::new();
        assert!(registry.self_id().is_none());
    }
}

// =============================================================================
// PEER REGISTRY ADD PEER TESTS
// =============================================================================

mod peer_registry_add_peer {
    use super::*;

    #[test]
    fn add_peer_should_increase_length() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("1"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn add_peer_should_make_registry_non_empty() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("1"));
        assert!(!registry.is_empty());
    }

    #[test]
    fn add_peer_should_allow_retrieval_by_id() {
        let mut registry = PeerRegistry::new();
        let peer = make_peer("node1");
        registry.add_peer(peer);
        assert!(registry.get("node1").is_some());
    }

    #[test]
    fn add_multiple_peers_should_track_all() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("a"));
        registry.add_peer(make_peer("b"));
        registry.add_peer(make_peer("c"));
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn add_peer_with_same_id_should_overwrite() {
        let mut registry = PeerRegistry::new();
        let peer1 = make_peer_with_addresses("same", "tcp://a:1", "tcp://a:2");
        let peer2 = make_peer_with_addresses("same", "tcp://b:1", "tcp://b:2");

        registry.add_peer(peer1);
        registry.add_peer(peer2);

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get("same").unwrap().router_address, "tcp://b:1");
    }

    #[test]
    fn add_peer_should_reject_self_id() {
        let mut registry = PeerRegistry::new();
        registry.set_self_id("self");
        registry.add_peer(make_peer("self"));
        assert!(registry.is_empty());
    }

    #[test]
    fn add_peer_should_accept_non_self_ids() {
        let mut registry = PeerRegistry::new();
        registry.set_self_id("self");
        registry.add_peer(make_peer("other"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn add_peer_before_set_self_id_should_work() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("node"));
        registry.set_self_id("self");
        assert_eq!(registry.len(), 1);
    }
}

// =============================================================================
// PEER REGISTRY GET TESTS
// =============================================================================

mod peer_registry_get {
    use super::*;

    #[test]
    fn get_existing_peer_should_return_some() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("exists"));
        assert!(registry.get("exists").is_some());
    }

    #[test]
    fn get_non_existing_peer_should_return_none() {
        let registry = PeerRegistry::new();
        assert!(registry.get("missing").is_none());
    }

    #[test]
    fn get_should_return_correct_peer_data() {
        let mut registry = PeerRegistry::new();
        let peer = make_peer_with_addresses("x", "tcp://host:1", "tcp://host:2");
        registry.add_peer(peer);

        let retrieved = registry.get("x").unwrap();
        assert_eq!(retrieved.id, "x");
        assert_eq!(retrieved.router_address, "tcp://host:1");
    }

    #[test]
    fn get_mut_should_allow_modification() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("m"));

        if let Some(peer) = registry.get_mut("m") {
            peer.reported_latency = 5.5;
        }

        assert!((registry.get("m").unwrap().reported_latency - 5.5).abs() < f64::EPSILON);
    }

    #[test]
    fn get_mut_non_existing_should_return_none() {
        let mut registry = PeerRegistry::new();
        assert!(registry.get_mut("missing").is_none());
    }
}

// =============================================================================
// PEER REGISTRY REMOVE TESTS
// =============================================================================

mod peer_registry_remove {
    use super::*;

    #[test]
    fn remove_existing_peer_should_return_peer() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("r"));
        let removed = registry.remove("r");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "r");
    }

    #[test]
    fn remove_existing_peer_should_decrease_length() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("1"));
        registry.add_peer(make_peer("2"));
        registry.remove("1");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn remove_non_existing_should_return_none() {
        let mut registry = PeerRegistry::new();
        assert!(registry.remove("missing").is_none());
    }

    #[test]
    fn remove_should_make_peer_inaccessible() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("gone"));
        registry.remove("gone");
        assert!(registry.get("gone").is_none());
    }

    #[test]
    fn remove_all_peers_should_make_registry_empty() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("a"));
        registry.add_peer(make_peer("b"));
        registry.remove("a");
        registry.remove("b");
        assert!(registry.is_empty());
    }
}

// =============================================================================
// PEER REGISTRY ITERATION TESTS
// =============================================================================

mod peer_registry_iteration {
    use super::*;

    #[test]
    fn iter_empty_registry_should_yield_nothing() {
        let registry = PeerRegistry::new();
        assert_eq!(registry.iter().count(), 0);
    }

    #[test]
    fn iter_should_yield_all_peers() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("a"));
        registry.add_peer(make_peer("b"));
        registry.add_peer(make_peer("c"));
        assert_eq!(registry.iter().count(), 3);
    }

    #[test]
    fn peer_ids_should_return_all_ids() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("x"));
        registry.add_peer(make_peer("y"));

        let ids = registry.peer_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"x".to_string()));
        assert!(ids.contains(&"y".to_string()));
    }

    #[test]
    fn peer_ids_empty_registry_should_return_empty_vec() {
        let registry = PeerRegistry::new();
        assert!(registry.peer_ids().is_empty());
    }
}

// =============================================================================
// PEER REGISTRY SELECT RANDOM TESTS
// =============================================================================

mod peer_registry_select_random {
    use super::*;

    #[test]
    fn select_random_zero_should_return_empty() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("1"));
        let selected = registry.select_random(0);
        assert!(selected.is_empty());
    }

    #[test]
    fn select_random_should_return_requested_count() {
        let mut registry = PeerRegistry::new();
        for i in 0..10 {
            registry.add_peer(make_peer(&i.to_string()));
        }
        let selected = registry.select_random(5);
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn select_random_more_than_available_should_return_all() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("1"));
        registry.add_peer(make_peer("2"));
        let selected = registry.select_random(10);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn select_random_from_empty_should_return_empty() {
        let registry = PeerRegistry::new();
        let selected = registry.select_random(5);
        assert!(selected.is_empty());
    }

    #[test]
    fn select_random_should_return_unique_peers() {
        let mut registry = PeerRegistry::new();
        for i in 0..10 {
            registry.add_peer(make_peer(&i.to_string()));
        }
        let selected = registry.select_random(5);
        let ids: std::collections::HashSet<_> = selected.iter().map(|p| &p.id).collect();
        assert_eq!(ids.len(), 5, "selected peers should be unique");
    }

    #[test]
    fn select_random_exact_count_should_return_all() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("a"));
        registry.add_peer(make_peer("b"));
        registry.add_peer(make_peer("c"));
        let selected = registry.select_random(3);
        assert_eq!(selected.len(), 3);
    }
}

// =============================================================================
// PEER REGISTRY LATENCY TESTS
// =============================================================================

mod peer_registry_latency {
    use super::*;

    #[test]
    fn update_latency_should_set_peer_latency() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("p"));
        registry.update_latency("p", 2.5);
        assert!((registry.get("p").unwrap().reported_latency - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn update_latency_should_touch_peer() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("p"));
        assert!(registry.get("p").unwrap().last_seen.is_none());

        registry.update_latency("p", 1.0);
        assert!(registry.get("p").unwrap().last_seen.is_some());
    }

    #[test]
    fn update_latency_non_existing_peer_should_be_noop() {
        let mut registry = PeerRegistry::new();
        registry.update_latency("missing", 5.0); // Should not panic
    }

    #[test]
    fn average_latency_empty_registry_should_return_zero() {
        let registry = PeerRegistry::new();
        assert!((registry.average_latency() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn average_latency_single_peer_should_return_peer_latency() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("p"));
        registry.update_latency("p", 3.0);
        assert!((registry.average_latency() - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn average_latency_multiple_peers_should_compute_mean() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("a"));
        registry.add_peer(make_peer("b"));
        registry.add_peer(make_peer("c"));

        registry.update_latency("a", 1.0);
        registry.update_latency("b", 2.0);
        registry.update_latency("c", 3.0);

        // Average of 1, 2, 3 = 2.0
        assert!((registry.average_latency() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn average_latency_with_zero_latencies_should_still_compute() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("a"));
        registry.add_peer(make_peer("b"));

        registry.update_latency("a", 0.0);
        registry.update_latency("b", 4.0);

        // Average of 0, 4 = 2.0
        assert!((registry.average_latency() - 2.0).abs() < f64::EPSILON);
    }
}

// =============================================================================
// NETWORK ERROR TESTS
// =============================================================================

mod network_error_construction {
    use super::*;

    #[test]
    fn bind_error_should_store_message() {
        let err = NetworkError::Bind("address in use".into());
        assert!(err.to_string().contains("address in use"));
    }

    #[test]
    fn connect_error_should_store_message() {
        let err = NetworkError::Connect("connection refused".into());
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn subscribe_error_should_store_message() {
        let err = NetworkError::Subscribe("invalid topic".into());
        assert!(err.to_string().contains("invalid topic"));
    }

    #[test]
    fn send_error_should_store_message() {
        let err = NetworkError::Send("buffer full".into());
        assert!(err.to_string().contains("buffer full"));
    }

    #[test]
    fn recv_error_should_store_message() {
        let err = NetworkError::Recv("timeout".into());
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn peer_not_found_should_store_peer_id() {
        let err = NetworkError::PeerNotFound("missing_peer".into());
        assert!(err.to_string().contains("missing_peer"));
    }

    #[test]
    fn invalid_message_should_store_reason() {
        let err = NetworkError::InvalidMessage("malformed".into());
        assert!(err.to_string().contains("malformed"));
    }
}

mod network_error_display {
    use super::*;

    #[test]
    fn bind_display_should_include_prefix() {
        let err = NetworkError::Bind("details".into());
        assert!(err.to_string().starts_with("failed to bind"));
    }

    #[test]
    fn connect_display_should_include_prefix() {
        let err = NetworkError::Connect("details".into());
        assert!(err.to_string().starts_with("failed to connect"));
    }

    #[test]
    fn subscribe_display_should_include_prefix() {
        let err = NetworkError::Subscribe("details".into());
        assert!(err.to_string().starts_with("failed to subscribe"));
    }

    #[test]
    fn send_display_should_include_prefix() {
        let err = NetworkError::Send("details".into());
        assert!(err.to_string().starts_with("failed to send"));
    }

    #[test]
    fn recv_display_should_include_prefix() {
        let err = NetworkError::Recv("details".into());
        assert!(err.to_string().starts_with("failed to receive"));
    }

    #[test]
    fn peer_not_found_display_should_include_prefix() {
        let err = NetworkError::PeerNotFound("id".into());
        assert!(err.to_string().starts_with("peer not found"));
    }

    #[test]
    fn invalid_message_display_should_include_prefix() {
        let err = NetworkError::InvalidMessage("reason".into());
        assert!(err.to_string().starts_with("invalid message"));
    }
}

mod network_error_traits {
    use super::*;

    #[test]
    fn should_implement_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(NetworkError::Bind("test".into()));
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn should_implement_debug() {
        let err = NetworkError::Bind("test".into());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Bind"));
    }
}

// =============================================================================
// RACER NETWORK CONSTRUCTION TESTS
// =============================================================================

mod racer_network_construction {
    use super::*;

    #[tokio::test]
    async fn new_should_accept_string_slices() {
        let _network = RacerNetwork::new("tcp://127.0.0.1:5000", "tcp://127.0.0.1:5001");
        // Construction should not panic
    }

    #[tokio::test]
    async fn new_should_accept_owned_strings() {
        let router = String::from("tcp://127.0.0.1:5000");
        let publisher = String::from("tcp://127.0.0.1:5001");
        let _network = RacerNetwork::new(router, publisher);
        // Construction should not panic
    }

    #[tokio::test]
    async fn new_should_accept_mixed_types() {
        let router = "tcp://127.0.0.1:5000";
        let publisher = String::from("tcp://127.0.0.1:5001");
        let _network = RacerNetwork::new(router, publisher);
        // Construction should not panic
    }
}

// =============================================================================
// RACER NETWORK TOPIC SUBSCRIPTION TESTS (without actual networking)
// =============================================================================

mod racer_network_topic_subscription {
    use super::*;

    #[tokio::test]
    async fn is_subscribed_should_return_false_initially() {
        let network = RacerNetwork::new("tcp://127.0.0.1:6000", "tcp://127.0.0.1:6001");
        assert!(!network.is_subscribed("any_topic").await);
    }

    // Note: Full subscription tests require ZeroMQ connections.
    // These tests focus on internal state management that we can verify.
}

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

mod integration_tests {
    use super::*;

    #[test]
    fn peer_info_clone_should_produce_independent_copy() {
        let mut peer1 = make_peer("original");
        let peer2 = peer1.clone();

        peer1.reported_latency = 10.0;

        assert!((peer2.reported_latency - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn registry_workflow_add_update_remove() {
        let mut registry = PeerRegistry::new();
        registry.set_self_id("me");

        // Add peers
        registry.add_peer(make_peer("peer1"));
        registry.add_peer(make_peer("peer2"));
        registry.add_peer(make_peer("peer3"));
        assert_eq!(registry.len(), 3);

        // Update latencies
        registry.update_latency("peer1", 1.0);
        registry.update_latency("peer2", 2.0);
        registry.update_latency("peer3", 3.0);

        // Check average
        assert!((registry.average_latency() - 2.0).abs() < f64::EPSILON);

        // Remove one peer
        registry.remove("peer2");
        assert_eq!(registry.len(), 2);

        // New average: (1 + 3) / 2 = 2.0
        assert!((registry.average_latency() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn select_random_should_not_include_removed_peers() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("keep1"));
        registry.add_peer(make_peer("keep2"));
        registry.add_peer(make_peer("remove"));

        registry.remove("remove");

        let selected = registry.select_random(10);
        for peer in selected {
            assert_ne!(peer.id, "remove");
        }
    }
}
