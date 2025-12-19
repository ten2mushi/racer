use ecdsa::signature::{Signer, Verifier};
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::keys::PublicKey;

#[derive(Clone, PartialEq, Eq)]
pub struct EcdsaSignature {
    bytes: Vec<u8>,
}

impl EcdsaSignature {
    pub fn from_der(bytes: &[u8]) -> Result<Self, SignatureError> {
        Signature::from_der(bytes).map_err(|_| SignatureError::InvalidSignature)?;
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }

    pub fn to_der(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&self.bytes)
    }

    pub fn from_base64(b64: &str) -> Result<Self, SignatureError> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|_| SignatureError::InvalidSignature)?;
        Self::from_der(&bytes)
    }

    fn to_signature(&self) -> Result<Signature, SignatureError> {
        Signature::from_der(&self.bytes).map_err(|_| SignatureError::InvalidSignature)
    }
}

impl std::fmt::Debug for EcdsaSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EcdsaSignature({}...)", &self.to_base64()[..16])
    }
}

impl Serialize for EcdsaSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_base64())
    }
}

impl<'de> Deserialize<'de> for EcdsaSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let b64 = String::deserialize(deserializer)?;
        Self::from_base64(&b64).map_err(serde::de::Error::custom)
    }
}

pub struct EcdsaSigner {
    signing_key: SigningKey,
}

impl EcdsaSigner {
    pub fn new(signing_key: SigningKey) -> Self {
        Self { signing_key }
    }

    pub fn sign(&self, message: &[u8]) -> EcdsaSignature {
        let signature: Signature = self.signing_key.sign(message);
        EcdsaSignature {
            bytes: signature.to_der().as_bytes().to_vec(),
        }
    }
}

pub struct EcdsaVerifier {
    verifying_key: VerifyingKey,
}

impl EcdsaVerifier {
    pub fn new(verifying_key: VerifyingKey) -> Self {
        Self { verifying_key }
    }

    pub fn from_public_key(public_key: &PublicKey) -> Result<Self, SignatureError> {
        let verifying_key = public_key
            .to_verifying_key()
            .map_err(|_| SignatureError::InvalidPublicKey)?;
        Ok(Self { verifying_key })
    }

    pub fn verify(&self, message: &[u8], signature: &EcdsaSignature) -> Result<(), SignatureError> {
        let sig = signature.to_signature()?;
        self.verifying_key
            .verify(message, &sig)
            .map_err(|_| SignatureError::VerificationFailed)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("invalid public key")]
    InvalidPublicKey,
    #[error("signature verification failed")]
    VerificationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    #[test]
    fn test_sign_verify() {
        let kp = KeyPair::generate();
        let signer = EcdsaSigner::new(kp.signing_key().clone());
        let verifier = EcdsaVerifier::new(kp.verifying_key());

        let message = b"test message";
        let signature = signer.sign(message);

        assert!(verifier.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_verify_wrong_message() {
        let kp = KeyPair::generate();
        let signer = EcdsaSigner::new(kp.signing_key().clone());
        let verifier = EcdsaVerifier::new(kp.verifying_key());

        let signature = signer.sign(b"original message");
        assert!(verifier.verify(b"different message", &signature).is_err());
    }

    #[test]
    fn test_signature_roundtrip() {
        let kp = KeyPair::generate();
        let signer = EcdsaSigner::new(kp.signing_key().clone());

        let signature = signer.sign(b"test");
        let b64 = signature.to_base64();
        let signature2 = EcdsaSignature::from_base64(&b64).unwrap();

        assert_eq!(signature, signature2);
    }
}
