use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::crypto::PublicKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub ecdsa_public: PublicKey,
    pub router_address: String,
    pub publisher_address: String,
    #[serde(skip)]
    pub reported_latency: f64,
    #[serde(skip)]
    pub last_seen: Option<Instant>,
}

impl PeerInfo {
    pub fn new(
        id: impl Into<String>,
        ecdsa_public: PublicKey,
        router_address: impl Into<String>,
        publisher_address: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            ecdsa_public,
            router_address: router_address.into(),
            publisher_address: publisher_address.into(),
            reported_latency: 0.0,
            last_seen: None,
        }
    }

    pub fn touch(&mut self) {
        self.last_seen = Some(Instant::now());
    }
}

#[derive(Debug, Default)]
pub struct PeerRegistry {
    peers: HashMap<String, PeerInfo>,
    self_id: Option<String>,
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_self_id(&mut self, id: impl Into<String>) {
        self.self_id = Some(id.into());
    }

    pub fn self_id(&self) -> Option<&str> {
        self.self_id.as_deref()
    }

    pub fn add_peer(&mut self, peer: PeerInfo) {
        if Some(peer.id.as_str()) == self.self_id.as_deref() {
            return;
        }
        self.peers.insert(peer.id.clone(), peer);
    }

    pub fn get(&self, id: &str) -> Option<&PeerInfo> {
        self.peers.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut PeerInfo> {
        self.peers.get_mut(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<PeerInfo> {
        self.peers.remove(id)
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &PeerInfo> {
        self.peers.values()
    }

    pub fn peer_ids(&self) -> Vec<String> {
        self.peers.keys().cloned().collect()
    }

    pub fn select_random(&self, n: usize) -> Vec<&PeerInfo> {
        use rand::seq::SliceRandom;
        let mut peers: Vec<_> = self.peers.values().collect();
        peers.shuffle(&mut rand::thread_rng());
        peers.into_iter().take(n).collect()
    }

    pub fn update_latency(&mut self, id: &str, latency: f64) {
        if let Some(peer) = self.peers.get_mut(id) {
            peer.reported_latency = latency;
            peer.touch();
        }
    }

    pub fn average_latency(&self) -> f64 {
        if self.peers.is_empty() {
            return 0.0;
        }

        let total: f64 = self.peers.values().map(|p| p.reported_latency).sum();
        total / self.peers.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    fn make_peer(id: &str) -> PeerInfo {
        PeerInfo::new(
            id,
            KeyPair::generate().public_key(),
            format!("tcp://127.0.0.1:2000{}", id),
            format!("tcp://127.0.0.1:2001{}", id),
        )
    }

    #[test]
    fn test_registry() {
        let mut registry = PeerRegistry::new();
        registry.add_peer(make_peer("1"));
        registry.add_peer(make_peer("2"));

        assert_eq!(registry.len(), 2);
        assert!(registry.get("1").is_some());
    }

    #[test]
    fn test_select_random() {
        let mut registry = PeerRegistry::new();
        for i in 0..10 {
            registry.add_peer(make_peer(&i.to_string()));
        }

        let selected = registry.select_random(3);
        assert_eq!(selected.len(), 3);
    }

    #[test]
    fn test_dont_add_self() {
        let mut registry = PeerRegistry::new();
        registry.set_self_id("self");
        registry.add_peer(make_peer("self"));

        assert!(registry.is_empty());
    }
}
