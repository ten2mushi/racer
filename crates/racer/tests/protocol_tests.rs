use std::collections::HashSet;
use std::time::Duration;

use racer::crypto::{EcdsaSigner, KeyPair};
use racer::protocol::{
    BatchedMessages, Echo, EchoType, GossipState, 
    ProtocolResponse, ProtocolResponseType, VectorClock,
};
use racer_core::message::DefaultMessage;

// =============================================================================
// VECTOR CLOCK TESTS
// =============================================================================

mod vector_clock_tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Construction Tests
    // -------------------------------------------------------------------------

    mod construction {
        use super::*;

        #[test]
        fn new_should_create_empty_clock() {
            let vc = VectorClock::new();
            assert!(vc.is_empty());
            assert_eq!(vc.len(), 0);
        }

        #[test]
        fn default_should_create_empty_clock() {
            let vc = VectorClock::default();
            assert!(vc.is_empty());
        }

        #[test]
        fn new_and_default_should_be_equal() {
            assert_eq!(VectorClock::new(), VectorClock::default());
        }
    }

    // -------------------------------------------------------------------------
    // Get Operation Tests
    // -------------------------------------------------------------------------

    mod get_operation {
        use super::*;

        #[test]
        fn get_should_return_zero_for_unknown_node() {
            let vc = VectorClock::new();
            assert_eq!(vc.get("unknown"), 0);
        }

        #[test]
        fn get_should_return_zero_for_empty_string_node() {
            let vc = VectorClock::new();
            assert_eq!(vc.get(""), 0);
        }

        #[test]
        fn get_should_return_set_value() {
            let mut vc = VectorClock::new();
            vc.set("node_a", 42);
            assert_eq!(vc.get("node_a"), 42);
        }

        #[test]
        fn get_should_handle_unicode_node_ids() {
            let mut vc = VectorClock::new();
            vc.set("节点A", 100);
            assert_eq!(vc.get("节点A"), 100);
        }
    }

    // -------------------------------------------------------------------------
    // Set Operation Tests
    // -------------------------------------------------------------------------

    mod set_operation {
        use super::*;

        #[test]
        fn set_should_store_value_for_new_node() {
            let mut vc = VectorClock::new();
            vc.set("node_a", 10);
            assert_eq!(vc.get("node_a"), 10);
            assert_eq!(vc.len(), 1);
        }

        #[test]
        fn set_should_overwrite_existing_value() {
            let mut vc = VectorClock::new();
            vc.set("node_a", 10);
            vc.set("node_a", 20);
            assert_eq!(vc.get("node_a"), 20);
            assert_eq!(vc.len(), 1);
        }

        #[test]
        fn set_should_handle_zero_value() {
            let mut vc = VectorClock::new();
            vc.set("node_a", 0);
            assert_eq!(vc.get("node_a"), 0);
            assert_eq!(vc.len(), 1); // Entry exists even if zero
        }

        #[test]
        fn set_should_handle_max_u64_value() {
            let mut vc = VectorClock::new();
            vc.set("node_a", u64::MAX);
            assert_eq!(vc.get("node_a"), u64::MAX);
        }

        #[test]
        fn set_should_handle_multiple_nodes() {
            let mut vc = VectorClock::new();
            vc.set("a", 1);
            vc.set("b", 2);
            vc.set("c", 3);
            assert_eq!(vc.len(), 3);
            assert_eq!(vc.get("a"), 1);
            assert_eq!(vc.get("b"), 2);
            assert_eq!(vc.get("c"), 3);
        }
    }

    // -------------------------------------------------------------------------
    // Increment Operation Tests
    // -------------------------------------------------------------------------

    mod increment_operation {
        use super::*;

        #[test]
        fn increment_should_start_at_one_for_new_node() {
            let mut vc = VectorClock::new();
            vc.increment("node_a");
            assert_eq!(vc.get("node_a"), 1);
        }

        #[test]
        fn increment_should_add_one_to_existing_value() {
            let mut vc = VectorClock::new();
            vc.set("node_a", 10);
            vc.increment("node_a");
            assert_eq!(vc.get("node_a"), 11);
        }

        #[test]
        fn increment_should_handle_multiple_increments() {
            let mut vc = VectorClock::new();
            for _ in 0..100 {
                vc.increment("node_a");
            }
            assert_eq!(vc.get("node_a"), 100);
        }

        #[test]
        fn increment_should_not_overflow_at_max_u64() {
            let mut vc = VectorClock::new();
            vc.set("node_a", u64::MAX);
            vc.increment("node_a");
            // saturating_add should prevent overflow
            assert_eq!(vc.get("node_a"), u64::MAX);
        }

        #[test]
        fn increment_should_be_independent_per_node() {
            let mut vc = VectorClock::new();
            vc.increment("a");
            vc.increment("a");
            vc.increment("b");
            assert_eq!(vc.get("a"), 2);
            assert_eq!(vc.get("b"), 1);
        }
    }

    // -------------------------------------------------------------------------
    // Merge Operation Tests
    // -------------------------------------------------------------------------

    mod merge_operation {
        use super::*;

        #[test]
        fn merge_should_take_max_for_common_nodes() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 5);

            let mut vc2 = VectorClock::new();
            vc2.set("a", 10);

            vc1.merge(&vc2);
            assert_eq!(vc1.get("a"), 10);
        }

        #[test]
        fn merge_should_keep_higher_value_from_self() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 10);

            let mut vc2 = VectorClock::new();
            vc2.set("a", 5);

            vc1.merge(&vc2);
            assert_eq!(vc1.get("a"), 10);
        }

        #[test]
        fn merge_should_add_nodes_from_other() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 1);

            let mut vc2 = VectorClock::new();
            vc2.set("b", 2);

            vc1.merge(&vc2);
            assert_eq!(vc1.get("a"), 1);
            assert_eq!(vc1.get("b"), 2);
            assert_eq!(vc1.len(), 2);
        }

        #[test]
        fn merge_with_empty_should_not_change_clock() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 5);
            let vc2 = VectorClock::new();

            vc1.merge(&vc2);
            assert_eq!(vc1.get("a"), 5);
            assert_eq!(vc1.len(), 1);
        }

        #[test]
        fn merge_empty_with_other_should_copy_other() {
            let mut vc1 = VectorClock::new();
            let mut vc2 = VectorClock::new();
            vc2.set("a", 5);
            vc2.set("b", 10);

            vc1.merge(&vc2);
            assert_eq!(vc1.get("a"), 5);
            assert_eq!(vc1.get("b"), 10);
        }

        #[test]
        fn merge_should_handle_complex_scenario() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 2);
            vc1.set("b", 1);

            let mut vc2 = VectorClock::new();
            vc2.set("a", 1);
            vc2.set("b", 3);
            vc2.set("c", 1);

            vc1.merge(&vc2);
            assert_eq!(vc1.get("a"), 2); // max(2, 1) = 2
            assert_eq!(vc1.get("b"), 3); // max(1, 3) = 3
            assert_eq!(vc1.get("c"), 1); // new from vc2
        }
    }

    // -------------------------------------------------------------------------
    // Happens-Before Relation Tests
    // -------------------------------------------------------------------------

    mod happens_before {
        use super::*;

        #[test]
        fn empty_clock_should_happen_before_non_empty() {
            let vc1 = VectorClock::new();
            let mut vc2 = VectorClock::new();
            vc2.set("a", 1);

            assert!(vc1.happens_before(&vc2));
        }

        #[test]
        fn non_empty_should_not_happen_before_empty() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 1);
            let vc2 = VectorClock::new();

            assert!(!vc1.happens_before(&vc2));
        }

        #[test]
        fn empty_should_not_happen_before_empty() {
            let vc1 = VectorClock::new();
            let vc2 = VectorClock::new();

            // Both empty - equal, not strictly less
            assert!(!vc1.happens_before(&vc2));
        }

        #[test]
        fn clock_should_not_happen_before_itself() {
            let mut vc = VectorClock::new();
            vc.set("a", 5);

            // A clock is not strictly less than itself
            assert!(!vc.happens_before(&vc));
        }

        #[test]
        fn strictly_less_should_happen_before() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 1);
            vc1.set("b", 1);

            let mut vc2 = VectorClock::new();
            vc2.set("a", 2);
            vc2.set("b", 2);

            assert!(vc1.happens_before(&vc2));
            assert!(!vc2.happens_before(&vc1));
        }

        #[test]
        fn single_node_increment_should_happen_before() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 1);

            let mut vc2 = VectorClock::new();
            vc2.set("a", 2);

            assert!(vc1.happens_before(&vc2));
        }

        #[test]
        fn neither_should_happen_before_for_concurrent() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 2);
            vc1.set("b", 1);

            let mut vc2 = VectorClock::new();
            vc2.set("a", 1);
            vc2.set("b", 2);

            assert!(!vc1.happens_before(&vc2));
            assert!(!vc2.happens_before(&vc1));
        }
    }

    // -------------------------------------------------------------------------
    // Concurrent Relation Tests
    // -------------------------------------------------------------------------

    mod concurrent {
        use super::*;

        #[test]
        fn concurrent_clocks_should_return_true() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 2);
            vc1.set("b", 1);

            let mut vc2 = VectorClock::new();
            vc2.set("a", 1);
            vc2.set("b", 2);

            assert!(vc1.concurrent(&vc2));
            assert!(vc2.concurrent(&vc1)); // Symmetric
        }

        #[test]
        fn ordered_clocks_should_not_be_concurrent() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 1);

            let mut vc2 = VectorClock::new();
            vc2.set("a", 2);

            assert!(!vc1.concurrent(&vc2));
        }

        #[test]
        fn equal_clocks_should_be_concurrent() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 5);

            let mut vc2 = VectorClock::new();
            vc2.set("a", 5);

            assert!(vc1.concurrent(&vc2));
        }

        #[test]
        fn empty_clocks_should_be_concurrent() {
            let vc1 = VectorClock::new();
            let vc2 = VectorClock::new();

            assert!(vc1.concurrent(&vc2));
        }
    }

    // -------------------------------------------------------------------------
    // Sum Operation Tests
    // -------------------------------------------------------------------------

    mod sum_operation {
        use super::*;

        #[test]
        fn sum_of_empty_clock_should_be_zero() {
            let vc = VectorClock::new();
            assert_eq!(vc.sum(), 0);
        }

        #[test]
        fn sum_should_add_all_entries() {
            let mut vc = VectorClock::new();
            vc.set("a", 10);
            vc.set("b", 20);
            vc.set("c", 30);
            assert_eq!(vc.sum(), 60);
        }

        #[test]
        fn sum_should_include_zero_valued_entries() {
            let mut vc = VectorClock::new();
            vc.set("a", 0);
            vc.set("b", 10);
            assert_eq!(vc.sum(), 10);
        }
    }

    // -------------------------------------------------------------------------
    // Nodes Iterator Tests
    // -------------------------------------------------------------------------

    mod nodes_iterator {
        use super::*;

        #[test]
        fn nodes_should_be_empty_for_empty_clock() {
            let vc = VectorClock::new();
            assert_eq!(vc.nodes().count(), 0);
        }

        #[test]
        fn nodes_should_return_all_node_ids() {
            let mut vc = VectorClock::new();
            vc.set("a", 1);
            vc.set("b", 2);
            vc.set("c", 3);

            let nodes: HashSet<_> = vc.nodes().collect();
            assert!(nodes.contains("a"));
            assert!(nodes.contains("b"));
            assert!(nodes.contains("c"));
            assert_eq!(nodes.len(), 3);
        }
    }

    // -------------------------------------------------------------------------
    // Display Trait Tests
    // -------------------------------------------------------------------------

    mod display {
        use super::*;

        #[test]
        fn display_empty_should_show_empty_braces() {
            let vc = VectorClock::new();
            assert_eq!(format!("{}", vc), "{}");
        }

        #[test]
        fn display_should_include_all_entries() {
            let mut vc = VectorClock::new();
            vc.set("a", 1);
            let display = format!("{}", vc);
            assert!(display.contains("a:1"));
        }
    }

    // -------------------------------------------------------------------------
    // Serialization Tests
    // -------------------------------------------------------------------------

    mod serialization {
        use super::*;

        #[test]
        fn should_serialize_to_json() {
            let mut vc = VectorClock::new();
            vc.set("a", 1);
            vc.set("b", 2);

            let json = serde_json::to_string(&vc).unwrap();
            assert!(json.contains("clock"));
        }

        #[test]
        fn should_roundtrip_through_json() {
            let mut vc1 = VectorClock::new();
            vc1.set("a", 1);
            vc1.set("b", 2);

            let json = serde_json::to_string(&vc1).unwrap();
            let vc2: VectorClock = serde_json::from_str(&json).unwrap();

            assert_eq!(vc1, vc2);
        }

        #[test]
        fn empty_clock_should_roundtrip() {
            let vc1 = VectorClock::new();
            let json = serde_json::to_string(&vc1).unwrap();
            let vc2: VectorClock = serde_json::from_str(&json).unwrap();
            assert_eq!(vc1, vc2);
        }
    }
}

// =============================================================================
// ECHO TYPE TESTS
// =============================================================================

mod echo_type_tests {
    use super::*;

    mod serialization {
        use super::*;

        #[test]
        fn echo_subscribe_should_serialize_to_snake_case() {
            let json = serde_json::to_string(&EchoType::EchoSubscribe).unwrap();
            assert_eq!(json, "\"echo_subscribe\"");
        }

        #[test]
        fn ready_subscribe_should_serialize_to_snake_case() {
            let json = serde_json::to_string(&EchoType::ReadySubscribe).unwrap();
            assert_eq!(json, "\"ready_subscribe\"");
        }

        #[test]
        fn should_deserialize_from_snake_case() {
            let parsed: EchoType = serde_json::from_str("\"echo_subscribe\"").unwrap();
            assert_eq!(parsed, EchoType::EchoSubscribe);
        }

        #[test]
        fn all_variants_should_roundtrip() {
            let variants = [
                EchoType::EchoSubscribe,
                EchoType::ReadySubscribe,
            ];

            for variant in variants {
                let json = serde_json::to_string(&variant).unwrap();
                let parsed: EchoType = serde_json::from_str(&json).unwrap();
                assert_eq!(variant, parsed);
            }
        }
    }

    mod traits {
        use super::*;

        #[test]
        fn should_implement_debug() {
            let et = EchoType::EchoSubscribe;
            let debug = format!("{:?}", et);
            assert!(debug.contains("EchoSubscribe"));
        }

        #[test]
        fn should_implement_clone() {
            let et1 = EchoType::ReadySubscribe;
            let et2 = et1.clone();
            assert_eq!(et1, et2);
        }

        #[test]
        fn should_implement_copy() {
            let et1 = EchoType::EchoSubscribe;
            let et2 = et1; // Copy, not move
            assert_eq!(et1, et2);
        }

        #[test]
        fn variants_should_not_be_equal_to_different_variants() {
            assert_ne!(EchoType::EchoSubscribe, EchoType::ReadySubscribe);
        }
    }
}

// =============================================================================
// PROTOCOL RESPONSE TYPE TESTS
// =============================================================================

mod protocol_response_type_tests {
    use super::*;

    mod serialization {
        use super::*;

        #[test]
        fn echo_response_should_serialize_to_snake_case() {
            let json = serde_json::to_string(&ProtocolResponseType::EchoResponse).unwrap();
            assert_eq!(json, "\"echo_response\"");
        }

        #[test]
        fn ready_response_should_serialize_to_snake_case() {
            let json = serde_json::to_string(&ProtocolResponseType::ReadyResponse).unwrap();
            assert_eq!(json, "\"ready_response\"");
        }

        #[test]
        fn all_variants_should_roundtrip() {
            let variants = [
                ProtocolResponseType::EchoResponse,
                ProtocolResponseType::ReadyResponse,
            ];

            for variant in variants {
                let json = serde_json::to_string(&variant).unwrap();
                let parsed: ProtocolResponseType = serde_json::from_str(&json).unwrap();
                assert_eq!(variant, parsed);
            }
        }
    }
}

// =============================================================================
// ECHO MESSAGE TESTS
// =============================================================================

mod echo_tests {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn new_should_set_echo_type() {
            let kp = KeyPair::generate();
            let echo = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());
            assert_eq!(echo.echo_type, EchoType::EchoSubscribe);
        }

        #[test]
        fn new_should_set_topic() {
            let kp = KeyPair::generate();
            let echo = Echo::new(EchoType::EchoSubscribe, "my_topic", kp.public_key());
            assert_eq!(echo.topic, "my_topic");
        }

        #[test]
        fn new_should_set_sender() {
            let kp = KeyPair::generate();
            let pk = kp.public_key();
            let echo = Echo::new(EchoType::EchoSubscribe, "topic", pk.clone());
            assert_eq!(echo.sender, pk);
        }

        #[test]
        fn new_should_set_timestamp() {
            let kp = KeyPair::generate();
            let before = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            let echo = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());

            let after = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            assert!(echo.timestamp >= before);
            assert!(echo.timestamp <= after);
        }

        #[test]
        fn new_should_accept_string_topic() {
            let kp = KeyPair::generate();
            let topic = String::from("string_topic");
            let echo = Echo::new(EchoType::EchoSubscribe, topic, kp.public_key());
            assert_eq!(echo.topic, "string_topic");
        }

        #[test]
        fn new_should_have_no_signature() {
            let kp = KeyPair::generate();
            let echo = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());
            assert!(echo.signature.is_none());
            assert!(!echo.is_signed());
        }
    }

    mod signing {
        use super::*;

        #[test]
        fn signing_bytes_should_include_echo_type() {
            let kp = KeyPair::generate();
            let echo = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());
            let bytes = echo.signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("echo_type"));
        }

        #[test]
        fn signing_bytes_should_include_topic() {
            let kp = KeyPair::generate();
            let echo = Echo::new(EchoType::EchoSubscribe, "unique_topic", kp.public_key());
            let bytes = echo.signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("unique_topic"));
        }

        #[test]
        fn signing_bytes_should_include_sender() {
            let kp = KeyPair::generate();
            let echo = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());
            let bytes = echo.signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("sender"));
        }

        #[test]
        fn signing_bytes_should_include_timestamp() {
            let kp = KeyPair::generate();
            let echo = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());
            let bytes = echo.signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("timestamp"));
        }

        #[test]
        fn sign_should_set_signature() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let mut echo = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());

            assert!(echo.signature.is_none());
            echo.sign(&signer);
            assert!(echo.signature.is_some());
            assert!(echo.is_signed());
        }

        #[test]
        fn different_echo_types_should_produce_different_signing_bytes() {
            let kp = KeyPair::generate();
            let echo1 = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());
            let echo2 = Echo::new(EchoType::ReadySubscribe, "topic", kp.public_key());

            assert_ne!(echo1.signing_bytes(), echo2.signing_bytes());
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn should_serialize_to_json() {
            let kp = KeyPair::generate();
            let echo = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());
            let json = serde_json::to_string(&echo).unwrap();
            assert!(json.contains("echo_type"));
            assert!(json.contains("topic"));
        }

        #[test]
        fn should_roundtrip_through_json() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let mut echo1 = Echo::new(EchoType::EchoSubscribe, "topic", kp.public_key());
            echo1.sign(&signer);

            let json = serde_json::to_string(&echo1).unwrap();
            let echo2: Echo = serde_json::from_str(&json).unwrap();

            assert_eq!(echo1.echo_type, echo2.echo_type);
            assert_eq!(echo1.topic, echo2.topic);
            assert_eq!(echo1.sender, echo2.sender);
        }
    }
}

// =============================================================================
// PROTOCOL RESPONSE TESTS
// =============================================================================

mod protocol_response_tests {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn echo_response_should_set_correct_type() {
            let kp = KeyPair::generate();
            let resp = ProtocolResponse::echo_response("topic123", kp.public_key());
            assert_eq!(resp.response_type, ProtocolResponseType::EchoResponse);
        }

        #[test]
        fn ready_response_should_set_correct_type() {
            let kp = KeyPair::generate();
            let resp = ProtocolResponse::ready_response("topic123", kp.public_key());
            assert_eq!(resp.response_type, ProtocolResponseType::ReadyResponse);
        }

        #[test]
        fn should_set_topic() {
            let kp = KeyPair::generate();
            let resp = ProtocolResponse::echo_response("my_topic", kp.public_key());
            assert_eq!(resp.topic, "my_topic");
        }

        #[test]
        fn should_set_sender() {
            let kp = KeyPair::generate();
            let pk = kp.public_key();
            let resp = ProtocolResponse::echo_response("topic", pk.clone());
            assert_eq!(resp.sender, pk);
        }

        #[test]
        fn should_set_timestamp() {
            let kp = KeyPair::generate();
            let before = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            let resp = ProtocolResponse::echo_response("topic", kp.public_key());

            let after = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            assert!(resp.timestamp >= before);
            assert!(resp.timestamp <= after);
        }

        #[test]
        fn new_should_have_no_signature() {
            let kp = KeyPair::generate();
            let resp = ProtocolResponse::echo_response("topic", kp.public_key());
            assert!(resp.signature.is_none());
        }
    }

    mod signing {
        use super::*;

        #[test]
        fn sign_should_set_signature() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let mut resp = ProtocolResponse::echo_response("topic", kp.public_key());

            assert!(resp.signature.is_none());
            resp.sign(&signer);
            assert!(resp.signature.is_some());
        }

        #[test]
        fn signing_bytes_should_include_response_type() {
            let kp = KeyPair::generate();
            let resp = ProtocolResponse::echo_response("topic", kp.public_key());
            let bytes = resp.signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("response_type"));
        }

        #[test]
        fn signing_bytes_should_include_topic() {
            let kp = KeyPair::generate();
            let resp = ProtocolResponse::echo_response("unique_topic", kp.public_key());
            let bytes = resp.signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("unique_topic"));
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn should_serialize_to_json() {
            let kp = KeyPair::generate();
            let resp = ProtocolResponse::echo_response("topic", kp.public_key());
            let json = serde_json::to_string(&resp).unwrap();

            assert!(json.contains("echo_response"));
            assert!(json.contains("topic"));
        }

        #[test]
        fn should_roundtrip_through_json() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let mut resp1 = ProtocolResponse::ready_response("topic", kp.public_key());
            resp1.sign(&signer);

            let json = serde_json::to_string(&resp1).unwrap();
            let resp2: ProtocolResponse = serde_json::from_str(&json).unwrap();

            assert_eq!(resp1.response_type, resp2.response_type);
            assert_eq!(resp1.topic, resp2.topic);
            assert_eq!(resp1.sender, resp2.sender);
        }
    }

    mod sender_id {
        use super::*;

        #[test]
        fn should_return_first_10_chars_of_public_key_hex() {
            let kp = KeyPair::generate();
            let resp = ProtocolResponse::echo_response("topic", kp.public_key());
            let sender_id = resp.sender_id();
            
            assert_eq!(sender_id.len(), 10);
            assert_eq!(sender_id, &resp.sender.to_hex()[..10]);
        }
    }
}

// =============================================================================
// GOSSIP ROUND TESTS
// =============================================================================

mod gossip_round_tests {
    use super::*;
    use racer::protocol::gossip::GossipRound;

    mod construction {
        use super::*;

        #[test]
        fn new_should_set_hash() {
            let round = GossipRound::new("hash123");
            assert_eq!(round.hash, "hash123");
        }

        #[test]
        fn new_should_accept_string() {
            let round = GossipRound::new(String::from("hash123"));
            assert_eq!(round.hash, "hash123");
        }

        #[test]
        fn new_should_initialize_empty_echo_waiting() {
            let round = GossipRound::new("hash");
            assert!(round.echo_waiting.is_empty());
        }

        #[test]
        fn new_should_initialize_empty_echo_received() {
            let round = GossipRound::new("hash");
            assert!(round.echo_received.is_empty());
        }

        #[test]
        fn new_should_initialize_empty_ready_waiting() {
            let round = GossipRound::new("hash");
            assert!(round.ready_waiting.is_empty());
        }

        #[test]
        fn new_should_initialize_empty_ready_received() {
            let round = GossipRound::new("hash");
            assert!(round.ready_received.is_empty());
        }

        #[test]
        fn new_should_set_echo_complete_to_false() {
            let round = GossipRound::new("hash");
            assert!(!round.echo_complete);
        }

        #[test]
        fn new_should_set_ready_complete_to_false() {
            let round = GossipRound::new("hash");
            assert!(!round.ready_complete);
        }

        #[test]
        fn new_should_set_delivered_to_false() {
            let round = GossipRound::new("hash");
            assert!(!round.delivered);
        }

        #[test]
        fn new_should_record_start_time() {
            let before = std::time::Instant::now();
            let round = GossipRound::new("hash");
            let after = std::time::Instant::now();

            // started_at should be between before and after
            assert!(round.started_at >= before);
            assert!(round.started_at <= after);
        }
    }

    mod record_echo {
        use super::*;

        #[test]
        fn should_remove_peer_from_waiting() {
            let mut round = GossipRound::new("hash");
            round.echo_waiting.insert("peer1".to_string());

            round.record_echo("peer1");

            assert!(!round.echo_waiting.contains("peer1"));
        }

        #[test]
        fn should_add_peer_to_received() {
            let mut round = GossipRound::new("hash");
            round.echo_waiting.insert("peer1".to_string());

            round.record_echo("peer1");

            assert!(round.echo_received.contains("peer1"));
        }

        #[test]
        fn should_handle_peer_not_in_waiting() {
            let mut round = GossipRound::new("hash");
            round.record_echo("peer1");

            assert!(round.echo_received.contains("peer1"));
        }

        #[test]
        fn should_handle_multiple_peers() {
            let mut round = GossipRound::new("hash");
            round.echo_waiting.insert("peer1".to_string());
            round.echo_waiting.insert("peer2".to_string());
            round.echo_waiting.insert("peer3".to_string());

            round.record_echo("peer1");
            round.record_echo("peer2");

            assert!(!round.echo_waiting.contains("peer1"));
            assert!(!round.echo_waiting.contains("peer2"));
            assert!(round.echo_waiting.contains("peer3"));
            assert!(round.echo_received.contains("peer1"));
            assert!(round.echo_received.contains("peer2"));
        }
    }

    mod record_ready {
        use super::*;

        #[test]
        fn should_remove_peer_from_waiting() {
            let mut round = GossipRound::new("hash");
            round.ready_waiting.insert("peer1".to_string());

            round.record_ready("peer1");

            assert!(!round.ready_waiting.contains("peer1"));
        }

        #[test]
        fn should_add_peer_to_received() {
            let mut round = GossipRound::new("hash");
            round.ready_waiting.insert("peer1".to_string());

            round.record_ready("peer1");

            assert!(round.ready_received.contains("peer1"));
        }
    }

    mod timeout {
        use super::*;

        #[test]
        fn elapsed_should_return_duration_since_start() {
            let round = GossipRound::new("hash");
            std::thread::sleep(Duration::from_millis(10));
            assert!(round.elapsed() >= Duration::from_millis(10));
        }

        #[test]
        fn is_timed_out_should_return_false_when_not_timed_out() {
            let round = GossipRound::new("hash");
            assert!(!round.is_timed_out(Duration::from_secs(60)));
        }

        #[test]
        fn is_timed_out_should_return_true_when_timed_out() {
            let round = GossipRound::new("hash");
            std::thread::sleep(Duration::from_millis(15));
            assert!(round.is_timed_out(Duration::from_millis(10)));
        }
    }
}

// =============================================================================
// GOSSIP STATE TESTS
// =============================================================================

mod gossip_state_tests {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn new_should_have_no_active_rounds() {
            let state = GossipState::<DefaultMessage>::new();
            assert_eq!(state.active_rounds(), 0);
        }

        #[test]
        fn default_should_equal_new() {
            let state1 = GossipState::<DefaultMessage>::new();
            let state2 = GossipState::<DefaultMessage>::default();
            assert_eq!(state1.active_rounds(), state2.active_rounds());
        }
    }

    mod timeout_configuration {
        use super::*;

        #[test]
        fn set_timeout_should_update_timeout() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.set_timeout(Duration::from_secs(120));
            // We test this indirectly via cleanup behavior
        }

        #[test]
        fn set_max_delivered_should_limit_tracked_hashes() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.set_max_delivered(3);

            for i in 0..5 {
                state.start_round(format!("hash{}", i));
                state.mark_delivered(&format!("hash{}", i));
            }

            // Only the last 3 should be tracked as recently delivered
            assert!(!state.was_recently_delivered("hash0"));
            assert!(!state.was_recently_delivered("hash1"));
            assert!(state.was_recently_delivered("hash2"));
            assert!(state.was_recently_delivered("hash3"));
            assert!(state.was_recently_delivered("hash4"));
        }
    }

    mod round_management {
        use super::*;

        #[test]
        fn start_round_should_create_new_round() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.start_round("hash1");
            assert_eq!(state.active_rounds(), 1);
        }

        #[test]
        fn start_round_should_return_mutable_round() {
            let mut state = GossipState::<DefaultMessage>::new();
            let round = state.start_round("hash1");
            round.echo_complete = true;

            let round = state.get_round("hash1").unwrap();
            assert!(round.echo_complete);
        }

        #[test]
        fn start_round_twice_should_not_duplicate() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.start_round("hash1");
            state.start_round("hash1");
            assert_eq!(state.active_rounds(), 1);
        }

        #[test]
        fn get_round_should_return_existing_round() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.start_round("hash1");

            let round = state.get_round("hash1");
            assert!(round.is_some());
            assert_eq!(round.unwrap().hash, "hash1");
        }

        #[test]
        fn get_round_should_return_none_for_unknown() {
            let state = GossipState::<DefaultMessage>::new();
            assert!(state.get_round("unknown").is_none());
        }

        #[test]
        fn get_round_mut_should_allow_modification() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.start_round("hash1");

            if let Some(round) = state.get_round_mut("hash1") {
                round.delivered = true;
            }

            assert!(state.is_delivered("hash1"));
        }
    }

    mod message_storage {
        use super::*;

        #[test]
        fn has_message_should_return_false_for_unknown() {
            let state = GossipState::<DefaultMessage>::new();
            assert!(!state.has_message("unknown"));
        }

        #[test]
        fn get_message_should_return_none_for_unknown() {
            let state = GossipState::<DefaultMessage>::new();
            assert!(state.get_message("unknown").is_none());
        }
    }

    mod delivery_tracking {
        use super::*;

        #[test]
        fn mark_delivered_should_set_delivered_flag() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.start_round("hash1");
            state.mark_delivered("hash1");

            assert!(state.is_delivered("hash1"));
        }

        #[test]
        fn is_delivered_should_return_false_for_unknown() {
            let state = GossipState::<DefaultMessage>::new();
            assert!(!state.is_delivered("unknown"));
        }

        #[test]
        fn is_delivered_should_return_false_before_delivery() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.start_round("hash1");
            assert!(!state.is_delivered("hash1"));
        }

        #[test]
        fn was_recently_delivered_should_track_delivered_hashes() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.start_round("hash1");
            state.mark_delivered("hash1");

            assert!(state.was_recently_delivered("hash1"));
        }

        #[test]
        fn was_recently_delivered_should_return_false_for_unknown() {
            let state = GossipState::<DefaultMessage>::new();
            assert!(!state.was_recently_delivered("unknown"));
        }

        #[test]
        fn active_rounds_should_not_count_delivered() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.start_round("hash1");
            state.start_round("hash2");
            assert_eq!(state.active_rounds(), 2);

            state.mark_delivered("hash1");
            assert_eq!(state.active_rounds(), 1);
        }
    }

    mod cleanup {
        use super::*;

        #[test]
        fn cleanup_timed_out_should_return_empty_when_no_timeouts() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.start_round("hash1");
            let timed_out = state.cleanup_timed_out();
            assert!(timed_out.is_empty());
        }

        #[test]
        fn cleanup_timed_out_should_not_remove_delivered_rounds() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.set_timeout(Duration::from_millis(1));
            state.start_round("hash1");
            state.mark_delivered("hash1");

            std::thread::sleep(Duration::from_millis(10));

            let timed_out = state.cleanup_timed_out();
            assert!(timed_out.is_empty());
        }

        #[test]
        fn cleanup_timed_out_should_remove_timed_out_rounds() {
            let mut state = GossipState::<DefaultMessage>::new();
            state.set_timeout(Duration::from_millis(5));
            state.start_round("hash1");

            std::thread::sleep(Duration::from_millis(15));

            let timed_out = state.cleanup_timed_out();
            assert!(timed_out.contains(&"hash1".to_string()));
            assert!(state.get_round("hash1").is_none());
        }
    }
}

// =============================================================================
// BATCHED MESSAGES TESTS
// =============================================================================

mod batched_messages_tests {
    use super::*;

    fn create_test_batched_message() -> BatchedMessages<DefaultMessage> {
        let kp = KeyPair::generate();
        let signer = EcdsaSigner::new(kp.signing_key().clone());
        let msg = DefaultMessage::new();

        BatchedMessages {
            batch_id: "batch_123".to_string(),
            creator_ecdsa: kp.public_key(),
            sender_ecdsa: kp.public_key(),
            merkle_root: "merkle_root_hash".to_string(),
            batch_size: 1,
            messages: vec![msg],
            vector_clock: VectorClock::new(),
            creator_signature: None,
            sender_signature: None,
            created_at: 1000,
            #[cfg(feature = "bls")]
            creator_bls: None,
            #[cfg(feature = "bls")]
            aggregated_signature: None,
        }
    }

    mod compute_hash {
        use super::*;

        #[test]
        fn compute_hash_should_return_hex_string() {
            let bm = create_test_batched_message();
            let hash = bm.compute_hash();
            // SHA256 hex is 64 characters
            assert_eq!(hash.len(), 64);
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }

        #[test]
        fn compute_hash_should_be_deterministic() {
            let bm = create_test_batched_message();
            let hash1 = bm.compute_hash();
            let hash2 = bm.compute_hash();
            assert_eq!(hash1, hash2);
        }

        #[test]
        fn different_batch_ids_should_produce_different_hashes() {
            let mut bm1 = create_test_batched_message();
            let mut bm2 = create_test_batched_message();
            bm1.batch_id = "batch_1".to_string();
            bm2.batch_id = "batch_2".to_string();

            assert_ne!(bm1.compute_hash(), bm2.compute_hash());
        }
    }

    mod signing_bytes {
        use super::*;

        #[test]
        fn creator_signing_bytes_should_include_batch_id() {
            let bm = create_test_batched_message();
            let bytes = bm.creator_signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("batch_123"));
        }

        #[test]
        fn creator_signing_bytes_should_include_merkle_root() {
            let bm = create_test_batched_message();
            let bytes = bm.creator_signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("merkle_root"));
        }

        #[test]
        fn creator_signing_bytes_should_include_batch_size() {
            let bm = create_test_batched_message();
            let bytes = bm.creator_signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("batch_size"));
        }

        #[test]
        fn creator_signing_bytes_should_include_created_at() {
            let bm = create_test_batched_message();
            let bytes = bm.creator_signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("created_at"));
        }

        #[test]
        fn sender_signing_bytes_should_include_batch_id() {
            let bm = create_test_batched_message();
            let bytes = bm.sender_signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("batch_123"));
        }

        #[test]
        fn sender_signing_bytes_should_include_merkle_root() {
            let bm = create_test_batched_message();
            let bytes = bm.sender_signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("merkle_root"));
        }

        #[test]
        fn sender_signing_bytes_should_include_sender() {
            let bm = create_test_batched_message();
            let bytes = bm.sender_signing_bytes();
            let json = String::from_utf8(bytes).unwrap();
            assert!(json.contains("sender"));
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn should_serialize_to_json() {
            let bm = create_test_batched_message();
            let json = serde_json::to_string(&bm).unwrap();
            assert!(json.contains("batch_id"));
            assert!(json.contains("messages"));
        }

        #[test]
        fn should_roundtrip_through_json() {
            let bm1 = create_test_batched_message();
            let json = serde_json::to_string(&bm1).unwrap();
            let bm2: BatchedMessages<DefaultMessage> = serde_json::from_str(&json).unwrap();

            assert_eq!(bm1.batch_id, bm2.batch_id);
            assert_eq!(bm1.batch_size, bm2.batch_size);
            assert_eq!(bm1.created_at, bm2.created_at);
        }
    }
}
