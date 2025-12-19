use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct RsiIndicator {
    period: usize,
    values: VecDeque<f64>,
    prev_value: Option<f64>,
    avg_gain: f64,
    avg_loss: f64,
    periods_processed: usize,
}

impl RsiIndicator {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "period must be positive");
        Self {
            period,
            values: VecDeque::with_capacity(period + 1),
            prev_value: None,
            avg_gain: 0.0,
            avg_loss: 0.0,
            periods_processed: 0,
        }
    }

    pub fn next(&mut self, value: f64) -> f64 {
        self.values.push_back(value);
        if self.values.len() > self.period + 1 {
            self.values.pop_front();
        }

        if let Some(prev) = self.prev_value {
            let change = value - prev;
            let gain = change.max(0.0);
            let loss = (-change).max(0.0);

            if self.periods_processed < self.period {
                self.avg_gain += gain;
                self.avg_loss += loss;
                self.periods_processed += 1;

                if self.periods_processed == self.period {
                    self.avg_gain /= self.period as f64;
                    self.avg_loss /= self.period as f64;
                }
            } else {
                let n = self.period as f64;
                self.avg_gain = (self.avg_gain * (n - 1.0) + gain) / n;
                self.avg_loss = (self.avg_loss * (n - 1.0) + loss) / n;
            }
        }

        self.prev_value = Some(value);
        self.calculate_rsi()
    }

    pub fn value(&self) -> f64 {
        self.calculate_rsi()
    }

    pub fn is_overbought(&self, threshold: f64) -> bool {
        self.calculate_rsi() > threshold
    }

    pub fn is_oversold(&self, threshold: f64) -> bool {
        self.calculate_rsi() < threshold
    }

    pub fn reset(&mut self) {
        self.values.clear();
        self.prev_value = None;
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        self.periods_processed = 0;
    }

    fn calculate_rsi(&self) -> f64 {
        if self.periods_processed < self.period {
            return 50.0; // Neutral during warmup
        }

        if self.avg_loss.abs() < f64::EPSILON {
            return 100.0;
        }

        let rs = self.avg_gain / self.avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_uptrend() {
        let mut rsi = RsiIndicator::new(14);

        for i in 0..30 {
            rsi.next(100.0 + i as f64);
        }

        assert!(rsi.value() > 70.0);
    }

    #[test]
    fn test_rsi_downtrend() {
        let mut rsi = RsiIndicator::new(14);

        for i in 0..30 {
            rsi.next(100.0 - i as f64);
        }

        assert!(rsi.value() < 30.0);
    }

    #[test]
    fn test_rsi_neutral() {
        let mut rsi = RsiIndicator::new(14);

        for i in 0..5 {
            rsi.next(100.0 + (i % 2) as f64);
        }

        assert!((rsi.value() - 50.0).abs() < 10.0);
    }
}
