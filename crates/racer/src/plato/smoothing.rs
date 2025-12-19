use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct SavitzkyGolayFilter {
    window_size: usize,
    coefficients: Vec<f64>,
    buffer: VecDeque<f64>,
}

impl SavitzkyGolayFilter {
    pub fn new(window_size: usize) -> Self {
        let window_size = if window_size % 2 == 0 {
            window_size + 1 // Ensure odd
        } else {
            window_size
        };

        let coefficients = match window_size {
            3 => vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
            5 => vec![-3.0, 12.0, 17.0, 12.0, -3.0]
                .into_iter()
                .map(|c| c / 35.0)
                .collect(),
            7 => vec![-2.0, 3.0, 6.0, 7.0, 6.0, 3.0, -2.0]
                .into_iter()
                .map(|c| c / 21.0)
                .collect(),
            9 => vec![-21.0, 14.0, 39.0, 54.0, 59.0, 54.0, 39.0, 14.0, -21.0]
                .into_iter()
                .map(|c| c / 231.0)
                .collect(),
            11 => vec![
                -36.0, 9.0, 44.0, 69.0, 84.0, 89.0, 84.0, 69.0, 44.0, 9.0, -36.0,
            ]
            .into_iter()
            .map(|c| c / 429.0)
            .collect(),
            _ => {
                vec![1.0 / window_size as f64; window_size]
            }
        };

        Self {
            window_size,
            coefficients,
            buffer: VecDeque::with_capacity(window_size),
        }
    }

    pub fn next(&mut self, value: f64) -> f64 {
        self.buffer.push_back(value);
        if self.buffer.len() > self.window_size {
            self.buffer.pop_front();
        }

        self.calculate()
    }

    pub fn value(&self) -> f64 {
        self.calculate()
    }

    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= self.window_size
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    fn calculate(&self) -> f64 {
        if self.buffer.is_empty() {
            return 0.0;
        }

        if self.buffer.len() < self.window_size {
            return self.buffer.iter().sum::<f64>() / self.buffer.len() as f64;
        }

        self.buffer
            .iter()
            .zip(&self.coefficients)
            .map(|(v, c)| v * c)
            .sum()
    }

    pub fn latest(&self) -> Option<f64> {
        self.buffer.back().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoothing() {
        let mut filter = SavitzkyGolayFilter::new(5);

        let data = [100.0, 102.0, 98.0, 104.0, 96.0, 100.0, 98.0];
        let mut smoothed = Vec::new();

        for &value in &data {
            smoothed.push(filter.next(value));
        }

        let raw_variance: f64 = data.iter().map(|&v| (v - 100.0).powi(2)).sum::<f64>() / data.len() as f64;
        let smooth_variance: f64 = smoothed.iter().skip(4).map(|&v| (v - 100.0).powi(2)).sum::<f64>()
            / (smoothed.len() - 4) as f64;

        assert!(smooth_variance <= raw_variance + 1.0); // Allow small tolerance
    }

    #[test]
    fn test_steady_state() {
        let mut filter = SavitzkyGolayFilter::new(5);

        for _ in 0..10 {
            filter.next(100.0);
        }

        assert!((filter.value() - 100.0).abs() < 0.001);
    }
}
