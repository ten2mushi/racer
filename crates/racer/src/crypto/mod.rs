mod ecdsa;
mod keys;

#[cfg(feature = "bls")]
mod bls;

pub use self::ecdsa::{EcdsaSignature, EcdsaSigner, EcdsaVerifier};
pub use keys::{KeyPair, PublicKey};

#[cfg(feature = "bls")]
pub use self::bls::{BlsPublicKey, BlsSecretKey, BlsSignature};

pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(sha256(data))
}
