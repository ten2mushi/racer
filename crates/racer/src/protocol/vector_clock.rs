use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorClock {
    clock: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, node_id: &str) -> u64 {
        self.clock.get(node_id).copied().unwrap_or(0)
    }

    pub fn increment(&mut self, node_id: &str) {
        let entry = self.clock.entry(node_id.to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    pub fn set(&mut self, node_id: &str, time: u64) {
        self.clock.insert(node_id.to_string(), time);
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (node_id, &time) in &other.clock {
            let entry = self.clock.entry(node_id.clone()).or_insert(0);
            *entry = (*entry).max(time);
        }
    }

    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut dominated = true;
        let mut strictly_less = false;

        for (node_id, &time) in &self.clock {
            let other_time = other.get(node_id);
            if time > other_time {
                dominated = false;
                break;
            }
            if time < other_time {
                strictly_less = true;
            }
        }

        if dominated {
            for (node_id, &time) in &other.clock {
                if !self.clock.contains_key(node_id) && time > 0 {
                    strictly_less = true;
                    break;
                }
            }
        }

        dominated && strictly_less
    }

    pub fn concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    pub fn sum(&self) -> u64 {
        self.clock.values().sum()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &str> {
        self.clock.keys().map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.clock.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clock.is_empty()
    }
}

impl std::fmt::Display for VectorClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entries: Vec<_> = self
            .clock
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect();
        write!(f, "{{{}}}", entries.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment() {
        let mut vc = VectorClock::new();
        vc.increment("a");
        vc.increment("a");
        vc.increment("b");

        assert_eq!(vc.get("a"), 2);
        assert_eq!(vc.get("b"), 1);
        assert_eq!(vc.get("c"), 0);
    }

    #[test]
    fn test_merge() {
        let mut vc1 = VectorClock::new();
        vc1.set("a", 2);
        vc1.set("b", 1);

        let mut vc2 = VectorClock::new();
        vc2.set("a", 1);
        vc2.set("b", 3);
        vc2.set("c", 1);

        vc1.merge(&vc2);

        assert_eq!(vc1.get("a"), 2);
        assert_eq!(vc1.get("b"), 3);
        assert_eq!(vc1.get("c"), 1);
    }

    #[test]
    fn test_happens_before() {
        let mut vc1 = VectorClock::new();
        vc1.set("a", 1);

        let mut vc2 = VectorClock::new();
        vc2.set("a", 2);

        assert!(vc1.happens_before(&vc2));
        assert!(!vc2.happens_before(&vc1));
    }

    #[test]
    fn test_concurrent() {
        let mut vc1 = VectorClock::new();
        vc1.set("a", 2);
        vc1.set("b", 1);

        let mut vc2 = VectorClock::new();
        vc2.set("a", 1);
        vc2.set("b", 2);

        assert!(vc1.concurrent(&vc2));
    }
}
