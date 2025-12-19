use racer::crypto::{sha256, sha256_hex, EcdsaSignature, EcdsaSigner, EcdsaVerifier, KeyPair, PublicKey};

// =============================================================================
// SHA-256 HASHING TESTS
// =============================================================================

mod sha256_tests {
    use super::*;

    mod basic_hashing {
        use super::*;

        #[test]
        fn empty_input_should_produce_known_hash() {
            let result = sha256(b"");
            let hex = hex::encode(result);
            assert_eq!(
                hex,
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            );
        }

        #[test]
        fn should_return_32_byte_array() {
            let result = sha256(b"test");
            assert_eq!(result.len(), 32);
        }

        #[test]
        fn should_produce_deterministic_output() {
            let input = b"deterministic test input";
            let result1 = sha256(input);
            let result2 = sha256(input);
            assert_eq!(result1, result2);
        }

        #[test]
        fn different_inputs_should_produce_different_hashes() {
            let hash1 = sha256(b"input A");
            let hash2 = sha256(b"input B");
            assert_ne!(hash1, hash2);
        }

        #[test]
        fn single_byte_change_should_produce_completely_different_hash() {
            let hash1 = sha256(b"hello world");
            let hash2 = sha256(b"hello worle"); // Changed last character
            
            // Count differing bytes - should be many due to avalanche effect
            let differing_bytes = hash1.iter().zip(hash2.iter()).filter(|(a, b)| a != b).count();
            assert!(differing_bytes > 10, "avalanche effect should cascade changes");
        }

        #[test]
        fn known_test_vector_should_match_rfc_example() {
            // Test vector: SHA-256("abc") from FIPS 180-4
            let result = sha256(b"abc");
            let hex = hex::encode(result);
            assert_eq!(
                hex,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            );
        }

        #[test]
        fn large_input_should_hash_correctly() {
            let large_input = vec![0xABu8; 1_000_000];
            let result = sha256(&large_input);
            assert_eq!(result.len(), 32);
            // Should be deterministic
            assert_eq!(result, sha256(&large_input));
        }

        #[test]
        fn binary_data_should_hash_without_issue() {
            let binary_data: Vec<u8> = (0..=255).collect();
            let result = sha256(&binary_data);
            assert_eq!(result.len(), 32);
        }
    }

    mod sha256_hex_tests {
        use super::*;

        #[test]
        fn should_return_64_character_hex_string() {
            let result = sha256_hex(b"test");
            assert_eq!(result.len(), 64);
        }

        #[test]
        fn should_contain_only_hex_characters() {
            let result = sha256_hex(b"any input");
            assert!(result.chars().all(|c| c.is_ascii_hexdigit()));
        }

        #[test]
        fn should_use_lowercase_hex() {
            let result = sha256_hex(b"test");
            assert!(result.chars().all(|c| !c.is_ascii_uppercase()));
        }

        #[test]
        fn should_match_manual_hex_encoding_of_sha256() {
            let input = b"consistency check";
            let raw_hash = sha256(input);
            let hex_hash = sha256_hex(input);
            assert_eq!(hex::encode(raw_hash), hex_hash);
        }

        #[test]
        fn empty_input_should_produce_known_hex() {
            let result = sha256_hex(b"");
            assert_eq!(
                result,
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            );
        }
    }
}

// =============================================================================
// KEYPAIR TESTS
// =============================================================================

mod keypair_tests {
    use super::*;

    mod generation {
        use super::*;

        #[test]
        fn generate_should_create_valid_keypair() {
            let kp = KeyPair::generate();
            assert!(!kp.public_key().as_bytes().is_empty());
        }

        #[test]
        fn generate_should_produce_unique_keys_each_time() {
            let kp1 = KeyPair::generate();
            let kp2 = KeyPair::generate();
            assert_ne!(kp1.public_key(), kp2.public_key());
        }

        #[test]
        fn generate_should_create_33_byte_compressed_public_key() {
            // P-256 compressed public key is 33 bytes (02/03 prefix + 32 bytes)
            let kp = KeyPair::generate();
            assert_eq!(kp.public_key().as_bytes().len(), 33);
        }

        #[test]
        fn public_key_should_start_with_compression_prefix() {
            let kp = KeyPair::generate();
            let first_byte = kp.public_key().as_bytes()[0];
            // Compressed P-256 keys start with 0x02 or 0x03
            assert!(first_byte == 0x02 || first_byte == 0x03);
        }

        #[test]
        fn to_bytes_should_return_32_byte_secret_key() {
            let kp = KeyPair::generate();
            assert_eq!(kp.to_bytes().len(), 32);
        }

        #[test]
        fn to_bytes_should_not_return_all_zeros() {
            let kp = KeyPair::generate();
            let bytes = kp.to_bytes();
            assert!(!bytes.iter().all(|&b| b == 0));
        }
    }

    mod from_bytes {
        use super::*;

        #[test]
        fn valid_bytes_should_recreate_same_keypair() {
            let original = KeyPair::generate();
            let bytes = original.to_bytes();
            let restored = KeyPair::from_bytes(&bytes).unwrap();
            assert_eq!(original.public_key(), restored.public_key());
        }

        #[test]
        fn signing_key_should_be_preserved_through_roundtrip() {
            let kp1 = KeyPair::generate();
            let bytes = kp1.to_bytes();
            let kp2 = KeyPair::from_bytes(&bytes).unwrap();
            
            // Both should produce identical signatures
            let signer1 = EcdsaSigner::new(kp1.signing_key().clone());
            let signer2 = EcdsaSigner::new(kp2.signing_key().clone());
            
            let message = b"test message";
            let sig1 = signer1.sign(message);
            let sig2 = signer2.sign(message);
            
            // Due to deterministic signing with same key, signatures should match
            // Note: ECDSA can be non-deterministic, but p256 uses RFC 6979
            assert_eq!(sig1, sig2);
        }

        #[test]
        fn empty_bytes_should_fail() {
            let result = KeyPair::from_bytes(&[]);
            assert!(result.is_err());
        }

        #[test]
        fn short_bytes_should_fail() {
            let result = KeyPair::from_bytes(&[0u8; 16]);
            assert!(result.is_err());
        }

        #[test]
        fn long_bytes_should_fail() {
            let result = KeyPair::from_bytes(&[0u8; 64]);
            assert!(result.is_err());
        }

        #[test]
        fn all_zeros_should_fail() {
            // Zero is not a valid P-256 scalar
            let result = KeyPair::from_bytes(&[0u8; 32]);
            assert!(result.is_err());
        }

        #[test]
        fn all_ones_should_fail() {
            // 2^256 - 1 is larger than the P-256 curve order
            let result = KeyPair::from_bytes(&[0xFFu8; 32]);
            assert!(result.is_err());
        }

        #[test]
        fn valid_32_byte_scalar_should_succeed() {
            // Use a known-valid scalar within curve order
            let kp = KeyPair::generate();
            let valid_bytes = kp.to_bytes();
            assert!(KeyPair::from_bytes(&valid_bytes).is_ok());
        }
    }

    mod debug_trait {
        use super::*;

        #[test]
        fn debug_should_not_expose_secret_key() {
            let kp = KeyPair::generate();
            let debug_output = format!("{:?}", kp);
            
            // Should contain "KeyPair" and "public_key"
            assert!(debug_output.contains("KeyPair"));
            assert!(debug_output.contains("public_key"));
            
            // Should NOT contain the raw secret key bytes
            let secret_hex = hex::encode(kp.to_bytes());
            assert!(!debug_output.contains(&secret_hex));
        }

        #[test]
        fn debug_should_show_truncated_public_key() {
            let kp = KeyPair::generate();
            let debug_output = format!("{:?}", kp);
            // PublicKey debug shows first 16 hex chars
            assert!(debug_output.len() < 200, "debug output should be concise");
        }
    }
}

// =============================================================================
// PUBLIC KEY TESTS
// =============================================================================

mod public_key_tests {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn from_bytes_should_accept_valid_compressed_key() {
            let kp = KeyPair::generate();
            let pk = kp.public_key();
            let bytes = pk.as_bytes();
            let pk2 = PublicKey::from_bytes(bytes).unwrap();
            assert_eq!(pk2.as_bytes(), bytes);
        }

        #[test]
        fn from_bytes_should_reject_empty_input() {
            let result = PublicKey::from_bytes(&[]);
            assert!(result.is_err());
        }

        #[test]
        fn from_bytes_should_reject_wrong_length() {
            let result = PublicKey::from_bytes(&[0x02; 32]); // Should be 33 bytes
            assert!(result.is_err());
        }

        #[test]
        fn from_bytes_should_reject_invalid_prefix() {
            // Valid compressed keys start with 0x02 or 0x03
            let mut invalid = vec![0x04; 33]; // 0x04 is uncompressed prefix
            let result = PublicKey::from_bytes(&invalid);
            assert!(result.is_err());
            
            invalid[0] = 0x00; // Invalid prefix
            let result = PublicKey::from_bytes(&invalid);
            assert!(result.is_err());
        }

        #[test]
        fn from_bytes_should_reject_malformed_encoding() {
            // Invalid SEC1 encoding: length 33 but data that cannot decode
            // The p256 library does SEC1 point decompression which can fail
            // for certain byte patterns even with valid prefix
            
            // Try a known invalid pattern (x-coordinate that yields no valid y)
            // We'll use a random pattern that's statistically unlikely to be valid
            let invalid: [u8; 33] = [
                0x02, // Valid compressed prefix
                0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE,
                0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
                0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
            ];
            let result = PublicKey::from_bytes(&invalid);
            // Most random x-coordinates don't have a valid y on the curve
            assert!(result.is_err(), "random x-coordinate should likely fail decompression");
        }
    }

    mod hex_conversion {
        use super::*;

        #[test]
        fn to_hex_should_return_66_character_string() {
            let kp = KeyPair::generate();
            let hex = kp.public_key().to_hex();
            assert_eq!(hex.len(), 66); // 33 bytes * 2
        }

        #[test]
        fn to_hex_should_use_lowercase() {
            let kp = KeyPair::generate();
            let hex = kp.public_key().to_hex();
            assert!(hex.chars().all(|c| !c.is_ascii_uppercase()));
        }

        #[test]
        fn from_hex_should_roundtrip_correctly() {
            let kp = KeyPair::generate();
            let original = kp.public_key();
            let hex = original.to_hex();
            let restored = PublicKey::from_hex(&hex).unwrap();
            assert_eq!(original, restored);
        }

        #[test]
        fn from_hex_should_accept_uppercase() {
            let kp = KeyPair::generate();
            let hex = kp.public_key().to_hex().to_uppercase();
            let result = PublicKey::from_hex(&hex);
            assert!(result.is_ok());
        }

        #[test]
        fn from_hex_should_reject_invalid_hex_chars() {
            let result = PublicKey::from_hex("02gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg");
            assert!(result.is_err());
        }

        #[test]
        fn from_hex_should_reject_odd_length() {
            let result = PublicKey::from_hex("02aabbcc"); // Odd number would be invalid
            assert!(result.is_err());
        }

        #[test]
        fn from_hex_should_reject_empty_string() {
            let result = PublicKey::from_hex("");
            assert!(result.is_err());
        }
    }

    mod equality_and_hashing {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn same_key_should_be_equal() {
            let kp = KeyPair::generate();
            let pk1 = kp.public_key();
            let pk2 = kp.public_key();
            assert_eq!(pk1, pk2);
        }

        #[test]
        fn different_keys_should_not_be_equal() {
            let pk1 = KeyPair::generate().public_key();
            let pk2 = KeyPair::generate().public_key();
            assert_ne!(pk1, pk2);
        }

        #[test]
        fn clone_should_be_equal_to_original() {
            let pk = KeyPair::generate().public_key();
            let cloned = pk.clone();
            assert_eq!(pk, cloned);
        }

        #[test]
        fn should_be_usable_in_hashset() {
            let mut set = HashSet::new();
            let pk1 = KeyPair::generate().public_key();
            let pk2 = KeyPair::generate().public_key();
            
            set.insert(pk1.clone());
            set.insert(pk2.clone());
            set.insert(pk1.clone()); // Duplicate
            
            assert_eq!(set.len(), 2);
            assert!(set.contains(&pk1));
            assert!(set.contains(&pk2));
        }
    }

    mod display_and_debug {
        use super::*;

        #[test]
        fn display_should_show_full_hex() {
            let kp = KeyPair::generate();
            let pk = kp.public_key();
            let display = format!("{}", pk);
            assert_eq!(display, pk.to_hex());
        }

        #[test]
        fn debug_should_show_truncated_hex() {
            let kp = KeyPair::generate();
            let pk = kp.public_key();
            let debug = format!("{:?}", pk);
            assert!(debug.starts_with("PublicKey("));
            assert!(debug.ends_with(")"));
            // Should be truncated (not full 66 chars)
            assert!(debug.len() < 30);
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn serde_roundtrip_should_preserve_key() {
            let pk = KeyPair::generate().public_key();
            let json = serde_json::to_string(&pk).unwrap();
            let restored: PublicKey = serde_json::from_str(&json).unwrap();
            assert_eq!(pk, restored);
        }

        #[test]
        fn should_serialize_as_hex_string() {
            let pk = KeyPair::generate().public_key();
            let json = serde_json::to_string(&pk).unwrap();
            // Should be a quoted hex string
            let expected = format!("\"{}\"", pk.to_hex());
            assert_eq!(json, expected);
        }

        #[test]
        fn should_deserialize_from_hex_string() {
            let pk = KeyPair::generate().public_key();
            let hex = pk.to_hex();
            let json = format!("\"{}\"", hex);
            let restored: PublicKey = serde_json::from_str(&json).unwrap();
            assert_eq!(pk, restored);
        }

        #[test]
        fn invalid_hex_should_fail_deserialization() {
            let json = "\"not_valid_hex\"";
            let result: Result<PublicKey, _> = serde_json::from_str(json);
            assert!(result.is_err());
        }
    }

    mod to_verifying_key {
        use super::*;

        #[test]
        fn should_return_valid_verifying_key() {
            let kp = KeyPair::generate();
            let pk = kp.public_key();
            let vk = pk.to_verifying_key().unwrap();
            
            // Verify it matches the original
            assert_eq!(kp.verifying_key(), vk);
        }

        #[test]
        fn should_be_usable_for_verification() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let pk = kp.public_key();
            
            let message = b"test";
            let signature = signer.sign(message);
            
            let vk = pk.to_verifying_key().unwrap();
            let verifier = EcdsaVerifier::new(vk);
            
            assert!(verifier.verify(message, &signature).is_ok());
        }
    }
}

// =============================================================================
// ECDSA SIGNATURE TESTS
// =============================================================================

mod ecdsa_signature_tests {
    use super::*;

    mod from_der {
        use super::*;

        #[test]
        fn valid_der_signature_should_parse() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig = signer.sign(b"test");
            
            let der_bytes = sig.to_der();
            let restored = EcdsaSignature::from_der(der_bytes).unwrap();
            assert_eq!(sig, restored);
        }

        #[test]
        fn empty_bytes_should_fail() {
            let result = EcdsaSignature::from_der(&[]);
            assert!(result.is_err());
        }

        #[test]
        fn random_bytes_should_fail() {
            let result = EcdsaSignature::from_der(&[0xAB; 72]);
            assert!(result.is_err());
        }

        #[test]
        fn truncated_signature_should_fail() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig = signer.sign(b"test");
            
            let der_bytes = sig.to_der();
            let truncated = &der_bytes[..der_bytes.len() - 5];
            let result = EcdsaSignature::from_der(truncated);
            assert!(result.is_err());
        }
    }

    mod base64_conversion {
        use super::*;

        #[test]
        fn to_base64_should_be_reversible() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig = signer.sign(b"test");
            
            let b64 = sig.to_base64();
            let restored = EcdsaSignature::from_base64(&b64).unwrap();
            assert_eq!(sig, restored);
        }

        #[test]
        fn should_contain_only_valid_base64_chars() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig = signer.sign(b"test");
            
            let b64 = sig.to_base64();
            assert!(b64.chars().all(|c| {
                c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='
            }));
        }

        #[test]
        fn invalid_base64_should_fail() {
            let result = EcdsaSignature::from_base64("not!!valid@@base64");
            assert!(result.is_err());
        }

        #[test]
        fn valid_base64_but_invalid_der_should_fail() {
            use base64::Engine;
            let invalid_der = base64::engine::general_purpose::STANDARD.encode(&[0xAB; 72]);
            let result = EcdsaSignature::from_base64(&invalid_der);
            assert!(result.is_err());
        }
    }

    mod equality {
        use super::*;

        #[test]
        fn same_signature_should_be_equal() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig1 = signer.sign(b"test");
            let sig2 = signer.sign(b"test");
            // With RFC 6979 deterministic signing, these should be equal
            assert_eq!(sig1, sig2);
        }

        #[test]
        fn different_messages_should_produce_different_signatures() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig1 = signer.sign(b"message A");
            let sig2 = signer.sign(b"message B");
            assert_ne!(sig1, sig2);
        }

        #[test]
        fn different_keys_should_produce_different_signatures() {
            let kp1 = KeyPair::generate();
            let kp2 = KeyPair::generate();
            let signer1 = EcdsaSigner::new(kp1.signing_key().clone());
            let signer2 = EcdsaSigner::new(kp2.signing_key().clone());
            
            let message = b"same message";
            let sig1 = signer1.sign(message);
            let sig2 = signer2.sign(message);
            assert_ne!(sig1, sig2);
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn serde_roundtrip_should_preserve_signature() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig = signer.sign(b"test");
            
            let json = serde_json::to_string(&sig).unwrap();
            let restored: EcdsaSignature = serde_json::from_str(&json).unwrap();
            assert_eq!(sig, restored);
        }

        #[test]
        fn should_serialize_as_base64_string() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig = signer.sign(b"test");
            
            let json = serde_json::to_string(&sig).unwrap();
            let expected = format!("\"{}\"", sig.to_base64());
            assert_eq!(json, expected);
        }
    }

    mod debug {
        use super::*;

        #[test]
        fn debug_should_show_truncated_base64() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig = signer.sign(b"test");
            
            let debug = format!("{:?}", sig);
            assert!(debug.starts_with("EcdsaSignature("));
            assert!(debug.contains("..."));
            assert!(debug.len() < 50);
        }
    }
}

// =============================================================================
// ECDSA SIGNER TESTS
// =============================================================================

mod ecdsa_signer_tests {
    use super::*;

    mod signing {
        use super::*;

        #[test]
        fn should_produce_valid_signature() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig = signer.sign(b"test message");
            
            // Signature should be non-empty DER bytes
            assert!(!sig.to_der().is_empty());
        }

        #[test]
        fn should_sign_empty_message() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let sig = signer.sign(b"");
            assert!(!sig.to_der().is_empty());
        }

        #[test]
        fn should_sign_large_message() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let large_message = vec![0xAB; 1_000_000];
            let sig = signer.sign(&large_message);
            assert!(!sig.to_der().is_empty());
        }

        #[test]
        fn should_sign_binary_message() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let binary: Vec<u8> = (0..=255).collect();
            let sig = signer.sign(&binary);
            assert!(!sig.to_der().is_empty());
        }

        #[test]
        fn deterministic_signing_should_produce_same_signature() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            
            let message = b"deterministic test";
            let sig1 = signer.sign(message);
            let sig2 = signer.sign(message);
            
            // With RFC 6979, same key + message = same signature
            assert_eq!(sig1, sig2);
        }
    }
}

// =============================================================================
// ECDSA VERIFIER TESTS
// =============================================================================

mod ecdsa_verifier_tests {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn from_public_key_should_work() {
            let kp = KeyPair::generate();
            let pk = kp.public_key();
            let result = EcdsaVerifier::from_public_key(&pk);
            assert!(result.is_ok());
        }

        #[test]
        fn from_verifying_key_should_work() {
            let kp = KeyPair::generate();
            let vk = kp.verifying_key();
            let _verifier = EcdsaVerifier::new(vk);
        }
    }

    mod verification {
        use super::*;

        #[test]
        fn valid_signature_should_verify() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let verifier = EcdsaVerifier::new(kp.verifying_key());
            
            let message = b"test message";
            let sig = signer.sign(message);
            
            assert!(verifier.verify(message, &sig).is_ok());
        }

        #[test]
        fn wrong_message_should_fail_verification() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let verifier = EcdsaVerifier::new(kp.verifying_key());
            
            let sig = signer.sign(b"original");
            assert!(verifier.verify(b"tampered", &sig).is_err());
        }

        #[test]
        fn single_bit_change_should_fail_verification() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let verifier = EcdsaVerifier::new(kp.verifying_key());
            
            let message = b"test";
            let sig = signer.sign(message);
            
            // Change one bit in message
            let mut tampered = message.to_vec();
            tampered[0] ^= 0x01;
            
            assert!(verifier.verify(&tampered, &sig).is_err());
        }

        #[test]
        fn wrong_key_should_fail_verification() {
            let kp1 = KeyPair::generate();
            let kp2 = KeyPair::generate();
            
            let signer = EcdsaSigner::new(kp1.signing_key().clone());
            let verifier = EcdsaVerifier::new(kp2.verifying_key());
            
            let message = b"test";
            let sig = signer.sign(message);
            
            assert!(verifier.verify(message, &sig).is_err());
        }

        #[test]
        fn empty_message_signature_should_verify() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let verifier = EcdsaVerifier::new(kp.verifying_key());
            
            let sig = signer.sign(b"");
            assert!(verifier.verify(b"", &sig).is_ok());
        }

        #[test]
        fn large_message_signature_should_verify() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let verifier = EcdsaVerifier::new(kp.verifying_key());
            
            let large_message = vec![0xCD; 100_000];
            let sig = signer.sign(&large_message);
            assert!(verifier.verify(&large_message, &sig).is_ok());
        }
    }

    mod cross_verification {
        use super::*;

        #[test]
        fn signature_should_verify_with_recreated_verifier() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            
            let message = b"test";
            let sig = signer.sign(message);
            
            // Recreate verifier from serialized public key
            let pk_hex = kp.public_key().to_hex();
            let restored_pk = PublicKey::from_hex(&pk_hex).unwrap();
            let verifier = EcdsaVerifier::from_public_key(&restored_pk).unwrap();
            
            assert!(verifier.verify(message, &sig).is_ok());
        }

        #[test]
        fn signature_should_verify_after_serialization() {
            let kp = KeyPair::generate();
            let signer = EcdsaSigner::new(kp.signing_key().clone());
            let verifier = EcdsaVerifier::new(kp.verifying_key());
            
            let message = b"test";
            let sig = signer.sign(message);
            
            // Serialize and deserialize signature
            let json = serde_json::to_string(&sig).unwrap();
            let restored_sig: EcdsaSignature = serde_json::from_str(&json).unwrap();
            
            assert!(verifier.verify(message, &restored_sig).is_ok());
        }
    }
}

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

mod integration_tests {
    use super::*;

    #[test]
    fn full_signing_workflow_should_work() {
        // 1. Generate key pair
        let kp = KeyPair::generate();
        
        // 2. Create signer and verifier
        let signer = EcdsaSigner::new(kp.signing_key().clone());
        let verifier = EcdsaVerifier::from_public_key(&kp.public_key()).unwrap();
        
        // 3. Sign a message
        let message = b"Hello, RACER consensus!";
        let signature = signer.sign(message);
        
        // 4. Verify the signature
        assert!(verifier.verify(message, &signature).is_ok());
    }

    #[test]
    fn keypair_roundtrip_should_preserve_signing_capability() {
        // Generate and serialize key
        let original = KeyPair::generate();
        let secret_bytes = original.to_bytes();
        
        // Restore key
        let restored = KeyPair::from_bytes(&secret_bytes).unwrap();
        
        // Sign with original, verify with restored
        let signer = EcdsaSigner::new(original.signing_key().clone());
        let verifier = EcdsaVerifier::new(restored.verifying_key());
        
        let message = b"roundtrip test";
        let sig = signer.sign(message);
        
        assert!(verifier.verify(message, &sig).is_ok());
    }

    #[test]
    fn signed_message_should_be_verifiable_by_any_party() {
        // Alice generates key and signs
        let alice_kp = KeyPair::generate();
        let alice_signer = EcdsaSigner::new(alice_kp.signing_key().clone());
        let alice_public_key = alice_kp.public_key();
        
        let message = b"I, Alice, authorize this transaction";
        let signature = alice_signer.sign(message);
        
        // Serialize for transmission
        let pk_json = serde_json::to_string(&alice_public_key).unwrap();
        let sig_json = serde_json::to_string(&signature).unwrap();
        
        // Bob receives and verifies
        let received_pk: PublicKey = serde_json::from_str(&pk_json).unwrap();
        let received_sig: EcdsaSignature = serde_json::from_str(&sig_json).unwrap();
        
        let bob_verifier = EcdsaVerifier::from_public_key(&received_pk).unwrap();
        assert!(bob_verifier.verify(message, &received_sig).is_ok());
    }

    #[test]
    fn hash_then_sign_should_work() {
        let kp = KeyPair::generate();
        let signer = EcdsaSigner::new(kp.signing_key().clone());
        let verifier = EcdsaVerifier::new(kp.verifying_key());
        
        // Hash a large document
        let document = b"This is a very large document...".repeat(1000);
        let hash = sha256(&document);
        
        // Sign the hash
        let signature = signer.sign(&hash);
        
        // Verify against the same hash
        assert!(verifier.verify(&hash, &signature).is_ok());
        
        // Different document should fail
        let different_doc = b"Different document".repeat(1000);
        let different_hash = sha256(&different_doc);
        assert!(verifier.verify(&different_hash, &signature).is_err());
    }
}
