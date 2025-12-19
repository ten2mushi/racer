//! BLS signature aggregation (feature-gated).
//!
//! This module is only available when the `bls` feature is enabled.

use blst::min_pk::{AggregateSignature, PublicKey, SecretKey, Signature};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct BlsSecretKey {
    inner: SecretKey,
}

impl BlsSecretKey {
    pub fn generate() -> Self {
        let mut ikm = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut ikm);
        Self {
            inner: SecretKey::key_gen(&ikm, &[]).expect("key generation failed"),
        }
    }

    pub fn from_seed(seed: &[u8]) -> Self {
        Self {
            inner: SecretKey::key_gen(seed, &[]).expect("key generation from seed failed"),
        }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, BlsError> {
        let inner = SecretKey::from_bytes(bytes).map_err(|_| BlsError::InvalidSecretKey)?;
        Ok(Self { inner })
    }

    pub fn public_key(&self) -> BlsPublicKey {
        BlsPublicKey {
            inner: self.inner.sk_to_pk(),
        }
    }

    pub fn sign(&self, message: &[u8]) -> BlsSignature {
        let sig = self.inner.sign(message, b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_", &[]);
        BlsSignature { inner: sig }
    }
}

#[derive(Clone, Debug)]
pub struct BlsPublicKey {
    inner: PublicKey,
}

impl BlsPublicKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BlsError> {
        let inner = PublicKey::from_bytes(bytes).map_err(|_| BlsError::InvalidPublicKey)?;
        Ok(Self { inner })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes().to_vec()
    }

    pub fn verify(&self, signature: &BlsSignature, message: &[u8]) -> Result<(), BlsError> {
        let result = signature.inner.verify(
            true,
            message,
            b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_",
            &[],
            &self.inner,
            true,
        );

        if result == blst::BLST_ERROR::BLST_SUCCESS {
            Ok(())
        } else {
            Err(BlsError::VerificationFailed)
        }
    }

    pub fn verify_aggregate(
        public_keys: &[BlsPublicKey],
        messages: &[&[u8]],
        signature: &BlsSignature,
    ) -> Result<(), BlsError> {
        if public_keys.len() != messages.len() {
            return Err(BlsError::InvalidAggregationInput);
        }

        let pub_keys: Vec<&PublicKey> = public_keys.iter().map(|pk| &pk.inner).collect();
        let result = signature.inner.aggregate_verify(
            true,
            messages,
            b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_",
            &pub_keys,
            true,
        );

        if result == blst::BLST_ERROR::BLST_SUCCESS {
            Ok(())
        } else {
            Err(BlsError::VerificationFailed)
        }
    }
}

impl Serialize for BlsPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&hex::encode(self.to_bytes()))
    }
}

impl<'de> Deserialize<'de> for BlsPublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let bytes = hex::decode(hex_str).map_err(serde::de::Error::custom)?;
        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug)]
pub struct BlsSignature {
    inner: Signature,
}

impl BlsSignature {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BlsError> {
        let inner = Signature::from_bytes(bytes).map_err(|_| BlsError::InvalidSignature)?;
        Ok(Self { inner })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes().to_vec()
    }

    pub fn aggregate(signatures: &[BlsSignature]) -> Result<Self, BlsError> {
        if signatures.is_empty() {
            return Err(BlsError::EmptyAggregation);
        }

        let refs: Vec<&Signature> = signatures.iter().map(|s| &s.inner).collect();
        let agg = AggregateSignature::aggregate(&refs, true)
            .map_err(|_| BlsError::AggregationFailed)?;

        Ok(Self {
            inner: agg.to_signature(),
        })
    }
}

impl Serialize for BlsSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&hex::encode(self.to_bytes()))
    }
}

impl<'de> Deserialize<'de> for BlsSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let bytes = hex::decode(hex_str).map_err(serde::de::Error::custom)?;
        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BlsError {
    #[error("invalid secret key")]
    InvalidSecretKey,
    #[error("invalid public key")]
    InvalidPublicKey,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("cannot aggregate empty signature list")]
    EmptyAggregation,
    #[error("signature aggregation failed")]
    AggregationFailed,
    #[error("invalid input for aggregation verification")]
    InvalidAggregationInput,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod secret_key {
        use super::*;

        #[test]
        fn generate_should_create_valid_keys() {
            let sk = BlsSecretKey::generate();
            let _pk = sk.public_key();
        }

        #[test]
        fn generate_should_produce_unique_keys() {
            let sk1 = BlsSecretKey::generate();
            let sk2 = BlsSecretKey::generate();
            
            assert_ne!(sk1.public_key().to_bytes(), sk2.public_key().to_bytes());
        }

        #[test]
        fn from_seed_should_be_deterministic() {
            let seed = b"01234567890123456789012345678901"; // 32 bytes
            let sk1 = BlsSecretKey::from_seed(seed);
            let sk2 = BlsSecretKey::from_seed(seed);

            assert_eq!(sk1.public_key().to_bytes(), sk2.public_key().to_bytes());
        }

        #[test]
        fn from_seed_should_produce_different_keys_for_different_seeds() {
            let seed1 = b"01234567890123456789012345678901";
            let seed2 = b"01234567890123456789012345678902";
            let sk1 = BlsSecretKey::from_seed(seed1);
            let sk2 = BlsSecretKey::from_seed(seed2);

            assert_ne!(sk1.public_key().to_bytes(), sk2.public_key().to_bytes());
        }

        #[test]
        fn from_bytes_should_accept_valid_scalar() {
            let mut bytes = [0u8; 32];
            bytes[31] = 1; 
            
            let sk = BlsSecretKey::from_bytes(&bytes);
            assert!(sk.is_ok());
        }

        #[test]
        fn sign_should_produce_valid_signature() {
            let sk = BlsSecretKey::generate();
            let pk = sk.public_key();
            let message = b"unit test message";
            
            let signature = sk.sign(message);
            assert!(pk.verify(&signature, message).is_ok());
        }
    }

    mod public_key {
        use super::*;

        #[test]
        fn to_bytes_should_be_consistent() {
            let sk = BlsSecretKey::generate();
            let pk = sk.public_key();
            let bytes1 = pk.to_bytes();
            let bytes2 = pk.to_bytes();
            assert_eq!(bytes1, bytes2);
        }

        #[test]
        fn from_bytes_should_roundtrip() {
            let sk = BlsSecretKey::generate();
            let pk = sk.public_key();
            let bytes = pk.to_bytes();
            
            let decoded = BlsPublicKey::from_bytes(&bytes).expect("Failed to decode valid bytes");
            assert_eq!(pk.to_bytes(), decoded.to_bytes());
        }

        #[test]
        fn verify_should_return_ok_for_correct_signature() {
            let sk = BlsSecretKey::generate();
            let pk = sk.public_key();
            let msg = b"message";
            let sig = sk.sign(msg);
            
            assert!(pk.verify(&sig, msg).is_ok());
        }

        #[test]
        fn verify_should_return_error_for_wrong_message() {
            let sk = BlsSecretKey::generate();
            let pk = sk.public_key();
            let msg = b"message";
            let wrong_msg = b"wrong message";
            let sig = sk.sign(msg);
            
            assert!(matches!(
                pk.verify(&sig, wrong_msg),
                Err(BlsError::VerificationFailed)
            ));
        }

        #[test]
        fn verify_should_return_error_for_wrong_key() {
            let sk1 = BlsSecretKey::generate();
            let sk2 = BlsSecretKey::generate();
            let pk2 = sk2.public_key();
            let msg = b"message";
            let sig = sk1.sign(msg);
            
            assert!(matches!(
                pk2.verify(&sig, msg),
                Err(BlsError::VerificationFailed)
            ));
        }

        #[test]
        fn verify_aggregate_should_succeed_for_valid_inputs() {
            let sk1 = BlsSecretKey::generate();
            let sk2 = BlsSecretKey::generate();
            let pk1 = sk1.public_key();
            let pk2 = sk2.public_key();
            
            let msg1 = b"msg1";
            let msg2 = b"msg2";
            
            let sig1 = sk1.sign(msg1);
            let sig2 = sk2.sign(msg2);
            
            let agg_sig = BlsSignature::aggregate(&[sig1, sig2]).unwrap();
            
            assert!(BlsPublicKey::verify_aggregate(
                &[pk1, pk2],
                &[msg1, msg2],
                &agg_sig
            ).is_ok());
        }

        #[test]
        fn verify_aggregate_should_handle_single_message_multiple_signers() {
            let sk1 = BlsSecretKey::generate();
            let sk2 = BlsSecretKey::generate();
            let msg = b"same message";
            
            let sig1 = sk1.sign(msg);
            let sig2 = sk2.sign(msg);
            
            let agg_sig = BlsSignature::aggregate(&[sig1, sig2]).unwrap();
            
            // When aggregating signatures on the SAME message, the verifier typically 
            // expects the inputs to be listed explicitly or the API to handle the mapping.
            // verifying_aggregate takes &[&[u8]], so we provide the message twice.
            assert!(BlsPublicKey::verify_aggregate(
                &[sk1.public_key(), sk2.public_key()],
                &[msg, msg],
                &agg_sig
            ).is_ok());
        }

        #[test]
        fn verify_aggregate_should_fail_on_length_mismatch() {
            let sk = BlsSecretKey::generate();
            let pk = sk.public_key();
            let sig = sk.sign(b"msg");
            
            // 2 keys, 1 message
            assert!(matches!(
                BlsPublicKey::verify_aggregate(
                    &[pk.clone(), pk.clone()],
                    &[b"msg"],
                    &sig
                ),
                Err(BlsError::InvalidAggregationInput)
            ));
        }

        #[test]
        fn verify_aggregate_should_fail_if_one_signature_invalid() {
            let sk1 = BlsSecretKey::generate();
            let sk2 = BlsSecretKey::generate();
            
            let msg1 = b"msg1";
            let msg2 = b"msg2";
            
            let sig1 = sk1.sign(msg1);
            let sig2_invalid = sk1.sign(msg2); // signed by sk1, but will verify against sk2's pubkey
            
            let agg_sig = BlsSignature::aggregate(&[sig1, sig2_invalid]).unwrap();
            
            assert!(matches!(
                BlsPublicKey::verify_aggregate(
                    &[sk1.public_key(), sk2.public_key()],
                    &[msg1, msg2],
                    &agg_sig
                ),
                Err(BlsError::VerificationFailed)
            ));
        }
    }

    mod signature {
        use super::*;

        #[test]
        fn roundtrip_serialization_should_work() {
            let sk = BlsSecretKey::generate();
            let sig = sk.sign(b"msg");
            let bytes = sig.to_bytes();
            
            let decoded = BlsSignature::from_bytes(&bytes).expect("Failed to decode signature");
            assert_eq!(sig.to_bytes(), decoded.to_bytes());
        }

        #[test]
        fn aggregate_should_fail_on_empty_list() {
            assert!(matches!(
                BlsSignature::aggregate(&[]),
                Err(BlsError::EmptyAggregation)
            ));
        }

        #[test]
        fn aggregate_should_return_equivalent_to_single_for_one_item() {
            let sk = BlsSecretKey::generate();
            let sig = sk.sign(b"msg");
            
            let agg = BlsSignature::aggregate(&[sig.clone()]).unwrap();
            assert_eq!(agg.to_bytes(), sig.to_bytes());
        }

        #[test]
        fn aggregate_should_be_order_independent() {
            let sk = BlsSecretKey::generate();
            let sig1 = sk.sign(b"msg1");
            let sig2 = sk.sign(b"msg2");
            
            let agg1 = BlsSignature::aggregate(&[sig1.clone(), sig2.clone()]).unwrap();
            let agg2 = BlsSignature::aggregate(&[sig2, sig1]).unwrap();
            
            assert_eq!(agg1.to_bytes(), agg2.to_bytes());
        }
    }
}
