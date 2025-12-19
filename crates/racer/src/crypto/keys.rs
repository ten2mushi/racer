use p256::ecdsa::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    #[cfg(feature = "bls")]
    bls_secret: crate::crypto::BlsSecretKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        #[cfg(not(feature = "bls"))]
        {
            Self {
                signing_key: SigningKey::random(&mut OsRng),
            }
        }
        #[cfg(feature = "bls")]
        {
            Self {
                signing_key: SigningKey::random(&mut OsRng),
                bls_secret: crate::crypto::BlsSecretKey::generate(),
            }
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        // Validate length first to avoid panicking
        let bytes_arr: &[u8; 32] = bytes
            .try_into()
            .map_err(|_| KeyError::InvalidSecretKey)?;
        
        let signing_key =
            SigningKey::from_bytes(bytes_arr.into()).map_err(|_| KeyError::InvalidSecretKey)?;
        
        #[cfg(feature = "bls")]
        {
            let bls_secret = crate::crypto::BlsSecretKey::from_seed(bytes_arr);
            
            Ok(Self { signing_key, bls_secret })
        }
        #[cfg(not(feature = "bls"))]
        {
            Ok(Self { signing_key })
        }
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }
    
    #[cfg(feature = "bls")]
    pub fn bls_secret(&self) -> &crate::crypto::BlsSecretKey {
        &self.bls_secret
    }
    
    #[cfg(feature = "bls")]
    pub fn bls_public_key(&self) -> crate::crypto::BlsPublicKey {
        self.bls_secret.public_key()
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        *self.signing_key.verifying_key()
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey::from_verifying_key(&self.verifying_key())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }
}

impl std::fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyPair")
            .field("public_key", &self.public_key())
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PublicKey {
    bytes: Vec<u8>,
}

impl PublicKey {
    pub fn from_verifying_key(key: &VerifyingKey) -> Self {
        use p256::elliptic_curve::sec1::ToEncodedPoint;
        Self {
            bytes: key.to_encoded_point(true).as_bytes().to_vec(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        VerifyingKey::from_sec1_bytes(bytes).map_err(|_| KeyError::InvalidPublicKey)?;
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }

    pub fn to_verifying_key(&self) -> Result<VerifyingKey, KeyError> {
        VerifyingKey::from_sec1_bytes(&self.bytes).map_err(|_| KeyError::InvalidPublicKey)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self, KeyError> {
        let bytes = hex::decode(hex_str).map_err(|_| KeyError::InvalidPublicKey)?;
        Self::from_bytes(&bytes)
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey({})", &self.to_hex()[..16])
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        Self::from_hex(&hex_str).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("invalid secret key")]
    InvalidSecretKey,
    #[error("invalid public key")]
    InvalidPublicKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let kp = KeyPair::generate();
        let pk = kp.public_key();
        assert!(!pk.as_bytes().is_empty());
    }

    #[test]
    fn test_keypair_roundtrip() {
        let kp1 = KeyPair::generate();
        let bytes = kp1.to_bytes();
        let kp2 = KeyPair::from_bytes(&bytes).unwrap();
        assert_eq!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    fn test_public_key_hex() {
        let kp = KeyPair::generate();
        let pk = kp.public_key();
        let hex = pk.to_hex();
        let pk2 = PublicKey::from_hex(&hex).unwrap();
        assert_eq!(pk, pk2);
    }
}
