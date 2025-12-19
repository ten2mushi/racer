use serde::{Deserialize, Serialize};

use crate::crypto::{EcdsaSignature, EcdsaSigner, PublicKey};

use super::VectorClock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchedMessages<M> {
    pub batch_id: String,
    pub creator_ecdsa: PublicKey,
    pub sender_ecdsa: PublicKey,
    pub merkle_root: String,
    pub batch_size: usize,
    pub messages: Vec<M>,
    pub vector_clock: VectorClock,
    pub creator_signature: Option<EcdsaSignature>,
    pub sender_signature: Option<EcdsaSignature>,
    pub created_at: u64,
    #[cfg(feature = "bls")]
    pub creator_bls: Option<crate::crypto::BlsPublicKey>,
    #[cfg(feature = "bls")]
    pub aggregated_signature: Option<crate::crypto::BlsSignature>,
}

impl<M> BatchedMessages<M>
where
    M: Serialize + Clone,
{
    pub fn compute_hash(&self) -> String {
        use crate::crypto::sha256_hex;
        let stable_fields = (
            &self.batch_id,
            &self.creator_ecdsa,
            &self.merkle_root,
            &self.creator_signature,
        );
        let bytes = serde_json::to_vec(&stable_fields).unwrap_or_default();
        sha256_hex(&bytes)
    }

    pub fn creator_signing_bytes(&self) -> Vec<u8> {
        serde_json::json!({
            "batch_id": self.batch_id,
            "merkle_root": self.merkle_root,
            "batch_size": self.batch_size,
            "created_at": self.created_at,
        })
        .to_string()
        .into_bytes()
    }

    pub fn sender_signing_bytes(&self) -> Vec<u8> {
        serde_json::json!({
            "batch_id": self.batch_id,
            "merkle_root": self.merkle_root,
            "sender": self.sender_ecdsa.to_hex(),
        })
        .to_string()
        .into_bytes()
    }

    pub fn sign_as_creator(&mut self, signer: &EcdsaSigner) {
        self.creator_signature = Some(signer.sign(&self.creator_signing_bytes()));
    }

    pub fn sign_as_sender(&mut self, signer: &EcdsaSigner) {
        self.sender_signature = Some(signer.sign(&self.sender_signing_bytes()));
    }

    pub fn is_fully_signed(&self) -> bool {
        self.creator_signature.is_some() && self.sender_signature.is_some()
    }

    pub fn become_sender(&self, keys: &crate::crypto::KeyPair) -> Self {
        let mut new_bm = Self {
            batch_id: self.batch_id.clone(),
            creator_ecdsa: self.creator_ecdsa.clone(),
            sender_ecdsa: keys.public_key(),
            merkle_root: self.merkle_root.clone(),
            batch_size: self.batch_size,
            messages: self.messages.clone(),
            vector_clock: self.vector_clock.clone(),
            creator_signature: self.creator_signature.clone(),
            sender_signature: None,
            created_at: self.created_at,
            #[cfg(feature = "bls")]
            creator_bls: self.creator_bls.clone(),
            #[cfg(feature = "bls")]
            aggregated_signature: self.aggregated_signature.clone(),
        };
        let signer = EcdsaSigner::new(keys.signing_key().clone());
        new_bm.sign_as_sender(&signer);
        new_bm
    }

    pub fn verify_creator_signature(&self) -> bool {
        if let Some(signature) = &self.creator_signature {
            let verifier = match crate::crypto::EcdsaVerifier::from_public_key(&self.creator_ecdsa) {
                Ok(v) => v,
                Err(_) => return false,
            };
            verifier.verify(&self.creator_signing_bytes(), signature).is_ok()
        } else {
            false
        }
    }

    pub fn verify_sender_signature(&self) -> bool {
        if let Some(signature) = &self.sender_signature {
            let verifier = match crate::crypto::EcdsaVerifier::from_public_key(&self.sender_ecdsa) {
                Ok(v) => v,
                Err(_) => return false,
            };
            verifier.verify(&self.sender_signing_bytes(), signature).is_ok()
        } else {
            false
        }
    }

    #[cfg(feature = "bls")]
    pub fn verify_aggregated_signature(&self) -> bool {
        use crate::crypto::BlsPublicKey;

        let (creator_key, signature) = match (&self.creator_bls, &self.aggregated_signature) {
            (Some(k), Some(s)) => (k, s),
            _ => return true,
        };

        let messages_bytes: Vec<Vec<u8>> = self
            .messages
            .iter()
            .map(|m| serde_json::to_vec(m).unwrap_or_default())
            .collect();
        
        let public_keys = vec![creator_key.clone(); self.messages.len()];
        
        let messages_refs: Vec<&[u8]> = messages_bytes.iter().map(|v| v.as_slice()).collect();

        BlsPublicKey::verify_aggregate(&public_keys, &messages_refs, signature).is_ok()
    }
}

 impl<M> BatchedMessages<M> {

 }


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EchoType {
    EchoSubscribe,
    ReadySubscribe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Echo {
    pub echo_type: EchoType,
    pub topic: String,
    pub sender: PublicKey,
    pub signature: Option<EcdsaSignature>,
    pub timestamp: u64,
}

impl Echo {
    pub fn new(echo_type: EchoType, topic: impl Into<String>, sender: PublicKey) -> Self {
        Self {
            echo_type,
            topic: topic.into(),
            sender,
            signature: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        serde_json::json!({
            "echo_type": self.echo_type,
            "topic": self.topic,
            "sender": self.sender.to_hex(),
            "timestamp": self.timestamp,
        })
        .to_string()
        .into_bytes()
    }

    pub fn sign(&mut self, signer: &EcdsaSigner) {
        self.signature = Some(signer.sign(&self.signing_bytes()));
    }

    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    pub fn sender_id(&self) -> String {
        self.sender.to_hex()[..10].to_string()
    }

    pub fn verify(&self) -> bool {
        if let Some(signature) = &self.signature {
            let verifier = match crate::crypto::EcdsaVerifier::from_public_key(&self.sender) {
                Ok(v) => v,
                Err(_) => return false,
            };
            verifier.verify(&self.signing_bytes(), signature).is_ok()
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolResponseType {
    EchoResponse,
    ReadyResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolResponse {
    pub response_type: ProtocolResponseType,
    pub topic: String,
    pub sender: PublicKey,
    pub signature: Option<EcdsaSignature>,
    pub timestamp: u64,
}

impl ProtocolResponse {
    pub fn echo_response(topic: impl Into<String>, sender: PublicKey) -> Self {
        Self {
            response_type: ProtocolResponseType::EchoResponse,
            topic: topic.into(),
            sender,
            signature: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    pub fn ready_response(topic: impl Into<String>, sender: PublicKey) -> Self {
        Self {
            response_type: ProtocolResponseType::ReadyResponse,
            topic: topic.into(),
            sender,
            signature: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        serde_json::json!({
            "response_type": self.response_type,
            "topic": self.topic,
            "sender": self.sender.to_hex(),
            "timestamp": self.timestamp,
        })
        .to_string()
        .into_bytes()
    }

    pub fn sign(&mut self, signer: &EcdsaSigner) {
        self.signature = Some(signer.sign(&self.signing_bytes()));
    }

    pub fn sender_id(&self) -> String {
        self.sender.to_hex()[..10].to_string()
    }

    pub fn verify(&self) -> bool {
        if let Some(signature) = &self.signature {
            let verifier = match crate::crypto::EcdsaVerifier::from_public_key(&self.sender) {
                Ok(v) => v,
                Err(_) => return false,
            };
            verifier.verify(&self.signing_bytes(), signature).is_ok()
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerDiscovery {
    pub ecdsa_public_key: PublicKey,
    pub router_address: String,
    pub publisher_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_type")]
pub enum ProtocolMessage<M> {
    #[serde(rename = "BatchedMessage")]
    BatchedMessages(BatchedMessages<M>),
    #[serde(rename = "Echo")]
    Echo(Echo),
    #[serde(rename = "Response")]
    Response(ProtocolResponse),
    #[serde(rename = "PeerDiscovery")]
    PeerDiscovery(PeerDiscovery),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CongestionUpdate {
    pub status: String,
    pub current_latency: f64,
    pub recently_missed: bool,
}

impl CongestionUpdate {
    pub fn new(current_latency: f64, recently_missed: bool) -> Self {
        Self {
            status: "CongestionUpdate".to_string(),
            current_latency,
            recently_missed,
        }
    }

    pub fn ok() -> Self {
        Self {
            status: "OK".to_string(),
            current_latency: 0.0,
            recently_missed: false,
        }
    }

    pub fn already_received() -> Self {
        Self {
            status: "ALREADY_RECEIVED".to_string(),
            current_latency: 0.0,
            recently_missed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_type_serialize() {
        let echo_type = EchoType::EchoSubscribe;
        let json = serde_json::to_string(&echo_type).unwrap();
        assert_eq!(json, "\"echo_subscribe\"");
    }

    #[test]
    fn test_protocol_response_types() {
        use crate::crypto::KeyPair;
        let keys = KeyPair::generate();
        
        let echo_resp = ProtocolResponse::echo_response("hash123", keys.public_key());
        assert_eq!(echo_resp.response_type, ProtocolResponseType::EchoResponse);
        
        let ready_resp = ProtocolResponse::ready_response("hash123", keys.public_key());
        assert_eq!(ready_resp.response_type, ProtocolResponseType::ReadyResponse);
    }

    #[test]
    fn test_protocol_message_serialization() {
        use crate::crypto::KeyPair;
        let keys = KeyPair::generate();
        
        let echo = Echo::new(EchoType::EchoSubscribe, "topic", keys.public_key());
        let msg: ProtocolMessage<String> = ProtocolMessage::Echo(echo);
        
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Echo"));
    }

    #[test]
    fn test_echo_verification() {
        use crate::crypto::{KeyPair, EcdsaSigner};
        let keys = KeyPair::generate();
        let signer = EcdsaSigner::new(keys.signing_key().clone());
        
        let mut echo = Echo::new(EchoType::EchoSubscribe, "test-topic", keys.public_key());
        
        assert!(!echo.verify());
        
        echo.sign(&signer);
        assert!(echo.verify());
        
        echo.topic = "tampered-topic".to_string();
        assert!(!echo.verify());
    }

    #[test]
    fn test_protocol_response_verification() {
        use crate::crypto::{KeyPair, EcdsaSigner};
        let keys = KeyPair::generate();
        let signer = EcdsaSigner::new(keys.signing_key().clone());
        
        let mut resp = ProtocolResponse::echo_response("test-topic", keys.public_key());
        
        assert!(!resp.verify());
        
        resp.sign(&signer);
        assert!(resp.verify());
        
        resp.topic = "tampered".to_string();
        assert!(!resp.verify());
    }

    #[test]
    fn test_batched_message_verification() {
        use crate::crypto::{KeyPair, EcdsaSigner};
        use crate::protocol::VectorClock;
        
        let creator_keys = KeyPair::generate();
        let creator_signer = EcdsaSigner::new(creator_keys.signing_key().clone());
        
        let sender_keys = KeyPair::generate();
        let sender_signer = EcdsaSigner::new(sender_keys.signing_key().clone());
        
        let mut bm = BatchedMessages {
            batch_id: "batch-1".to_string(),
            creator_ecdsa: creator_keys.public_key(),
            sender_ecdsa: sender_keys.public_key(),
            merkle_root: "root".to_string(),
            batch_size: 1,
            messages: vec!["msg1".to_string()],
            vector_clock: VectorClock::new(),
            creator_signature: None,
            sender_signature: None,
            created_at: 1000,
            #[cfg(feature = "bls")]
            creator_bls: None,
            #[cfg(feature = "bls")]
            aggregated_signature: None,
        };
        
        assert!(!bm.verify_creator_signature());
        assert!(!bm.verify_sender_signature());
        
        bm.sign_as_creator(&creator_signer);
        assert!(bm.verify_creator_signature());
        assert!(!bm.verify_sender_signature());
        
        bm.sign_as_sender(&sender_signer);
        assert!(bm.verify_creator_signature());
        assert!(bm.verify_sender_signature());
        
        bm.merkle_root = "tampered".to_string();
        assert!(!bm.verify_creator_signature());
        assert!(!bm.verify_sender_signature()); // Sender sig also covers merkle root
    }
}
