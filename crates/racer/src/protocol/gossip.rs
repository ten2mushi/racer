use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use crate::protocol::BatchedMessages;
use crate::Message;

#[derive(Debug)]
pub struct GossipRound {
    pub hash: String,
    pub started_at: Instant,
    pub echo_waiting: HashSet<String>,
    pub echo_received: HashSet<String>,
    pub ready_waiting: HashSet<String>,
    pub ready_received: HashSet<String>,
    pub echo_complete: bool,
    pub ready_complete: bool,
    pub delivered: bool,
}

impl GossipRound {
    pub fn new(hash: impl Into<String>) -> Self {
        Self {
            hash: hash.into(),
            started_at: Instant::now(),
            echo_waiting: HashSet::new(),
            echo_received: HashSet::new(),
            ready_waiting: HashSet::new(),
            ready_received: HashSet::new(),
            echo_complete: false,
            ready_complete: false,
            delivered: false,
        }
    }

    pub fn record_echo(&mut self, peer_id: &str) {
        self.echo_waiting.remove(peer_id);
        self.echo_received.insert(peer_id.to_string());
    }

    pub fn record_ready(&mut self, peer_id: &str) {
        self.ready_waiting.remove(peer_id);
        self.ready_received.insert(peer_id.to_string());
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        self.elapsed() > timeout
    }
}

pub struct GossipState<M: Message> {
    rounds: HashMap<String, GossipRound>,
    received_messages: HashMap<String, BatchedMessages<M>>,
    delivered_hashes: VecDeque<String>,
    max_delivered: usize,
    default_timeout: Duration,
}

impl<M: Message> GossipState<M> {
    pub fn new() -> Self {
        Self {
            rounds: HashMap::new(),
            received_messages: HashMap::new(),
            delivered_hashes: VecDeque::new(),
            max_delivered: 1000,
            default_timeout: Duration::from_secs(60),
        }
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }

    pub fn set_max_delivered(&mut self, max: usize) {
        self.max_delivered = max;
    }

    pub fn start_round(&mut self, hash: impl Into<String>) -> &mut GossipRound {
        let hash = hash.into();
        self.rounds.entry(hash.clone()).or_insert_with(|| GossipRound::new(&hash))
    }

    pub fn get_round(&self, hash: &str) -> Option<&GossipRound> {
        self.rounds.get(hash)
    }

    pub fn get_round_mut(&mut self, hash: &str) -> Option<&mut GossipRound> {
        self.rounds.get_mut(hash)
    }

    pub fn store_message(&mut self, hash: String, message: BatchedMessages<M>) {
        self.received_messages.insert(hash, message);
    }

    pub fn get_message(&self, hash: &str) -> Option<&BatchedMessages<M>> {
        self.received_messages.get(hash)
    }

    pub fn has_message(&self, hash: &str) -> bool {
        self.received_messages.contains_key(hash)
    }

    pub fn mark_delivered(&mut self, hash: &str) {
        if let Some(round) = self.rounds.get_mut(hash) {
            round.delivered = true;
        }

        self.delivered_hashes.push_back(hash.to_string());
        while self.delivered_hashes.len() > self.max_delivered {
            if let Some(old_hash) = self.delivered_hashes.pop_front() {
                self.rounds.remove(&old_hash);
                self.received_messages.remove(&old_hash);
            }
        }
    }

    pub fn is_delivered(&self, hash: &str) -> bool {
        self.rounds
            .get(hash)
            .map(|r| r.delivered)
            .unwrap_or(false)
    }

    pub fn was_recently_delivered(&self, hash: &str) -> bool {
        self.delivered_hashes.iter().any(|h| h == hash)
    }

    pub fn cleanup_timed_out(&mut self) -> Vec<String> {
        let timeout = self.default_timeout;
        let timed_out: Vec<_> = self
            .rounds
            .iter()
            .filter(|(_, round)| !round.delivered && round.is_timed_out(timeout))
            .map(|(hash, _)| hash.clone())
            .collect();

        for hash in &timed_out {
            self.rounds.remove(hash);
            self.received_messages.remove(hash);
        }

        timed_out
    }

    pub fn active_rounds(&self) -> usize {
        self.rounds.values().filter(|r| !r.delivered).count()
    }
}

impl<M: Message> Default for GossipState<M> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use racer_core::message::DefaultMessage;

    #[test]
    fn test_gossip_round() {
        let mut round = GossipRound::new("hash123");
        round.echo_waiting.insert("peer1".into());
        round.echo_waiting.insert("peer2".into());

        round.record_echo("peer1");
        assert!(!round.echo_waiting.contains("peer1"));
        assert!(round.echo_received.contains("peer1"));
    }

    #[test]
    fn test_gossip_state() {
        let mut state = GossipState::<DefaultMessage>::new();
        
        state.start_round("hash1");
        state.start_round("hash2");
        
        assert_eq!(state.active_rounds(), 2);
        
        state.mark_delivered("hash1");
        assert!(state.is_delivered("hash1"));
        assert!(!state.is_delivered("hash2"));
    }
}
