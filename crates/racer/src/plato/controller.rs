use std::collections::VecDeque;

use rand::Rng;

use crate::config::PlatoConfig;

use super::rsi::RsiIndicator;
use super::smoothing::SavitzkyGolayFilter;

#[derive(Debug)]
pub struct PlatoController {
    config: PlatoConfig,
    current_latency: f64,
    publish_frequency: f64,
    our_latency: VecDeque<f64>,
    peer_latency: VecDeque<f64>,
    our_rsi_up: RsiIndicator,
    peer_rsi_up: RsiIndicator,
    our_rsi_down: RsiIndicator,
    peer_rsi_down: RsiIndicator,
    our_savgol_up: SavitzkyGolayFilter,
    peer_savgol_up: SavitzkyGolayFilter,
    our_savgol_down: SavitzkyGolayFilter,
    peer_savgol_down: SavitzkyGolayFilter,
    max_samples: usize,
    recently_missed_delivery: bool,
    pub timing_changed: bool,
}

impl PlatoController {
    pub fn new(config: PlatoConfig) -> Self {
        Self {
            our_rsi_up: RsiIndicator::new(config.rsi_increase_period),
            peer_rsi_up: RsiIndicator::new(config.rsi_increase_period),
            our_rsi_down: RsiIndicator::new(config.rsi_decrease_period),
            peer_rsi_down: RsiIndicator::new(config.rsi_decrease_period),
            our_savgol_up: SavitzkyGolayFilter::new(config.savgol_increase_window),
            peer_savgol_up: SavitzkyGolayFilter::new(config.savgol_increase_window),
            our_savgol_down: SavitzkyGolayFilter::new(config.savgol_decrease_window),
            peer_savgol_down: SavitzkyGolayFilter::new(config.savgol_decrease_window),
            current_latency: config.target_latency_secs,
            publish_frequency: config.target_publishing_frequency_secs,
            our_latency: VecDeque::with_capacity(100),
            peer_latency: VecDeque::with_capacity(100),
            max_samples: 100,
            recently_missed_delivery: false,
            timing_changed: false,
            config,
        }
    }

    pub fn record_our_latency(&mut self, latency: f64) {
        self.our_latency.push_back(latency);
        if self.our_latency.len() > self.max_samples {
            self.our_latency.pop_front();
        }

        self.our_rsi_up.next(latency);
        self.our_rsi_down.next(latency);
        self.our_savgol_up.next(latency);
        self.our_savgol_down.next(latency);
    }

    pub fn record_peer_latency(&mut self, latency: f64) {
        self.peer_latency.push_back(latency);
        if self.peer_latency.len() > self.max_samples {
            self.peer_latency.pop_front();
        }

        self.peer_rsi_up.next(latency);
        self.peer_rsi_down.next(latency);
        self.peer_savgol_up.next(latency);
        self.peer_savgol_down.next(latency);
    }

    pub fn set_missed_delivery(&mut self, missed: bool) {
        self.recently_missed_delivery = missed;
    }

    pub fn check_increasing_congestion(&mut self) {
        if !self.our_savgol_up.is_ready() || !self.peer_savgol_up.is_ready() {
            return;
        }

        let weighted_latest = self.weighted_latency();
        let our_rsi = self.our_rsi_up.value();
        let peer_rsi = self.peer_rsi_up.value();

        if self.current_latency <= 0.5 * weighted_latest {
            let proposed = self.current_latency * 2.0;
            if proposed < self.config.max_gossip_timeout_secs * 0.85 {
                self.current_latency = proposed;
                self.timing_changed = true;
                tracing::debug!(
                    current_latency = self.current_latency,
                    "PLATO: fast-forward"
                );
            }
        }
        else if our_rsi > self.config.rsi_overbought && peer_rsi > self.config.rsi_overbought {
            let mut rng = rand::thread_rng();
            let increase = rng.gen_range(1.01..1.10);

            self.current_latency = (self.current_latency * increase).min(
                self.config.max_gossip_timeout_secs,
            );
            self.publish_frequency = (self.publish_frequency * increase).min(
                self.config.max_publishing_frequency_secs,
            );

            self.timing_changed = true;
            tracing::debug!(
                current_latency = self.current_latency,
                publish_frequency = self.publish_frequency,
                our_rsi,
                peer_rsi,
                "PLATO: throttling due to congestion"
            );
        }
    }

    pub fn check_decreasing_congestion(&mut self) {
        if !self.our_savgol_down.is_ready() || !self.peer_savgol_down.is_ready() {
            return;
        }

        let our_rsi = self.our_rsi_down.value();
        let peer_rsi = self.peer_rsi_down.value();

        if our_rsi < self.config.rsi_oversold && peer_rsi < self.config.rsi_oversold {
            let mut rng = rand::thread_rng();
            let decrease = rng.gen_range(0.90..0.99);

            self.current_latency = (self.current_latency * decrease)
                .max(self.config.minimum_latency_secs);
            self.publish_frequency = (self.publish_frequency * decrease)
                .max(self.config.minimum_latency_secs);

            self.timing_changed = true;
            tracing::debug!(
                current_latency = self.current_latency,
                publish_frequency = self.publish_frequency,
                our_rsi,
                peer_rsi,
                "PLATO: accelerating due to low congestion"
            );
        }
    }

    pub fn current_latency(&self) -> f64 {
        self.current_latency
    }

    pub fn publish_frequency(&self) -> f64 {
        self.publish_frequency
    }

    pub fn weighted_latency(&self) -> f64 {
        let our_smoothed = self.our_savgol_up.value();
        let peer_smoothed = self.peer_savgol_up.value();

        let w = self.config.own_latency_weight;
        w * our_smoothed + (1.0 - w) * peer_smoothed
    }

    pub fn recently_missed_delivery(&self) -> bool {
        self.recently_missed_delivery
    }

    pub fn clear_timing_changed(&mut self) {
        self.timing_changed = false;
    }

    pub fn stats(&self) -> PlatoStats {
        PlatoStats {
            current_latency: self.current_latency,
            publish_frequency: self.publish_frequency,
            our_rsi_up: self.our_rsi_up.value(),
            our_rsi_down: self.our_rsi_down.value(),
            peer_rsi_up: self.peer_rsi_up.value(),
            peer_rsi_down: self.peer_rsi_down.value(),
            our_latency_samples: self.our_latency.len(),
            peer_latency_samples: self.peer_latency.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlatoStats {
    pub current_latency: f64,
    pub publish_frequency: f64,
    pub our_rsi_up: f64,
    pub our_rsi_down: f64,
    pub peer_rsi_up: f64,
    pub peer_rsi_down: f64,
    pub our_latency_samples: usize,
    pub peer_latency_samples: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_creation() {
        let config = PlatoConfig::default();
        let controller = PlatoController::new(config.clone());

        assert!((controller.current_latency() - config.target_latency_secs).abs() < 0.001);
    }

    #[test]
    fn test_record_latency() {
        let config = PlatoConfig::default();
        let mut controller = PlatoController::new(config);

        for i in 0..20 {
            controller.record_our_latency(2.0 + i as f64 * 0.1);
            controller.record_peer_latency(2.0 + i as f64 * 0.1);
        }

        let stats = controller.stats();
        assert_eq!(stats.our_latency_samples, 20);
        assert_eq!(stats.peer_latency_samples, 20);
    }
}
