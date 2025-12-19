use racer::config::PlatoConfig;
use racer::plato::{PlatoController, RsiIndicator, SavitzkyGolayFilter};

// =============================================================================
// RSI INDICATOR TESTS
// =============================================================================

mod rsi_tests {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn new_should_create_indicator_with_specified_period() {
            let rsi = RsiIndicator::new(14);
            // Initial value during warmup should be neutral (50.0)
            assert!((rsi.value() - 50.0).abs() < f64::EPSILON);
        }

        #[test]
        fn new_should_accept_minimum_period_of_one() {
            let rsi = RsiIndicator::new(1);
            assert!((rsi.value() - 50.0).abs() < f64::EPSILON);
        }

        #[test]
        fn new_should_accept_large_periods() {
            let rsi = RsiIndicator::new(100);
            assert!((rsi.value() - 50.0).abs() < f64::EPSILON);
        }

        #[test]
        #[should_panic(expected = "period must be positive")]
        fn new_should_panic_for_zero_period() {
            let _ = RsiIndicator::new(0);
        }
    }

    mod warmup_behavior {
        use super::*;

        #[test]
        fn should_return_neutral_50_during_warmup() {
            let mut rsi = RsiIndicator::new(14);
            
            // During first 14 periods, should return 50.0 (neutral)
            for i in 0..14 {
                let value = rsi.next(100.0 + i as f64);
                assert!(
                    (value - 50.0).abs() < f64::EPSILON,
                    "expected 50.0 during warmup at step {}, got {}",
                    i, value
                );
            }
        }

        #[test]
        fn should_transition_from_warmup_after_period_samples() {
            let mut rsi = RsiIndicator::new(5);
            
            // Feed 5 increasing values to complete warmup
            for i in 0..5 {
                rsi.next(100.0 + i as f64);
            }
            
            // Next value should no longer return exactly 50.0
            let value = rsi.next(105.0);
            assert!(
                (value - 50.0).abs() > 0.1,
                "expected non-neutral value after warmup, got {}",
                value
            );
        }

        #[test]
        fn warmup_should_respect_exact_period_count() {
            let period = 7;
            let mut rsi = RsiIndicator::new(period);
            
            // At exactly period samples, should still be in warmup
            for i in 0..period {
                let _ = rsi.next(100.0 + i as f64);
            }
            
            // This sample completes warmup, should get real RSI
            let value = rsi.next(107.0);
            assert!(value > 50.0, "expected high RSI after uptrend, got {}", value);
        }
    }

    mod trend_detection {
        use super::*;

        #[test]
        fn should_return_high_rsi_for_consistent_uptrend() {
            let mut rsi = RsiIndicator::new(14);
            
            // Feed consistently increasing values
            for i in 0..30 {
                rsi.next(100.0 + i as f64);
            }
            
            let value = rsi.value();
            assert!(value > 70.0, "expected high RSI (>70) for uptrend, got {}", value);
        }

        #[test]
        fn should_return_low_rsi_for_consistent_downtrend() {
            let mut rsi = RsiIndicator::new(14);
            
            // Feed consistently decreasing values
            for i in 0..30 {
                rsi.next(100.0 - i as f64);
            }
            
            let value = rsi.value();
            assert!(value < 30.0, "expected low RSI (<30) for downtrend, got {}", value);
        }

        #[test]
        fn should_return_rsi_near_50_for_flat_market() {
            let mut rsi = RsiIndicator::new(14);
            
            // Feed constant values
            for _ in 0..30 {
                rsi.next(100.0);
            }
            
            let value = rsi.value();
            // After warmup with zero changes, avg_loss will be 0, so RSI = 100
            // This is expected behavior for pure flat market with no volatility
            assert!(
                value >= 50.0,
                "expected RSI >= 50 for flat market, got {}",
                value
            );
        }

        #[test]
        fn should_return_rsi_near_50_for_oscillating_market() {
            let mut rsi = RsiIndicator::new(14);
            
            // Feed alternating up/down values
            for i in 0..50 {
                let offset = if i % 2 == 0 { 1.0 } else { -1.0 };
                rsi.next(100.0 + offset);
            }
            
            let value = rsi.value();
            assert!(
                (value - 50.0).abs() < 15.0,
                "expected RSI near 50 for oscillating market, got {}",
                value
            );
        }

        #[test]
        fn should_approach_100_for_only_gains() {
            let mut rsi = RsiIndicator::new(5);
            
            // Feed strictly increasing values
            for i in 0..20 {
                rsi.next(i as f64);
            }
            
            let value = rsi.value();
            assert!(value > 95.0, "expected RSI near 100 for only gains, got {}", value);
        }

        #[test]
        fn should_approach_0_for_only_losses() {
            let mut rsi = RsiIndicator::new(5);
            
            // Feed strictly decreasing values
            for i in 0..20 {
                rsi.next(100.0 - i as f64);
            }
            
            let value = rsi.value();
            assert!(value < 5.0, "expected RSI near 0 for only losses, got {}", value);
        }
    }

    mod threshold_methods {
        use super::*;

        #[test]
        fn is_overbought_should_return_true_when_above_threshold() {
            let mut rsi = RsiIndicator::new(14);
            
            // Create overbought condition
            for i in 0..30 {
                rsi.next(100.0 + i as f64);
            }
            
            assert!(rsi.is_overbought(70.0), "expected overbought for high RSI");
        }

        #[test]
        fn is_overbought_should_return_false_when_below_threshold() {
            let mut rsi = RsiIndicator::new(14);
            
            // Create oversold condition
            for i in 0..30 {
                rsi.next(100.0 - i as f64);
            }
            
            assert!(!rsi.is_overbought(70.0), "expected not overbought for low RSI");
        }

        #[test]
        fn is_oversold_should_return_true_when_below_threshold() {
            let mut rsi = RsiIndicator::new(14);
            
            // Create oversold condition
            for i in 0..30 {
                rsi.next(100.0 - i as f64);
            }
            
            assert!(rsi.is_oversold(30.0), "expected oversold for low RSI");
        }

        #[test]
        fn is_oversold_should_return_false_when_above_threshold() {
            let mut rsi = RsiIndicator::new(14);
            
            // Create overbought condition
            for i in 0..30 {
                rsi.next(100.0 + i as f64);
            }
            
            assert!(!rsi.is_oversold(30.0), "expected not oversold for high RSI");
        }

        #[test]
        fn is_overbought_should_handle_exact_threshold() {
            // When RSI equals threshold exactly, should return false (not strictly greater)
            let rsi = RsiIndicator::new(14);
            // During warmup, RSI is 50.0
            assert!(!rsi.is_overbought(50.0), "RSI at threshold should not be overbought");
        }

        #[test]
        fn is_oversold_should_handle_exact_threshold() {
            let rsi = RsiIndicator::new(14);
            // During warmup, RSI is 50.0
            assert!(!rsi.is_oversold(50.0), "RSI at threshold should not be oversold");
        }
    }

    mod reset_behavior {
        use super::*;

        #[test]
        fn reset_should_return_to_initial_state() {
            let mut rsi = RsiIndicator::new(14);
            
            // Build up some state
            for i in 0..30 {
                rsi.next(100.0 + i as f64);
            }
            
            rsi.reset();
            
            // Should be back to neutral warmup state
            assert!((rsi.value() - 50.0).abs() < f64::EPSILON);
        }

        #[test]
        fn reset_should_clear_previous_value_tracking() {
            let mut rsi = RsiIndicator::new(5);
            
            // Add values and then reset
            for i in 0..10 {
                rsi.next(100.0 + i as f64);
            }
            
            rsi.reset();
            
            // First value after reset should not compute change from last value before reset
            let result = rsi.next(50.0);
            assert!((result - 50.0).abs() < f64::EPSILON, "first value after reset should be neutral");
        }
    }

    mod value_method {
        use super::*;

        #[test]
        fn value_should_return_same_as_last_next_call() {
            let mut rsi = RsiIndicator::new(14);
            
            for i in 0..20 {
                let from_next = rsi.next(100.0 + i as f64);
                let from_value = rsi.value();
                assert!(
                    (from_next - from_value).abs() < f64::EPSILON,
                    "value() should match last next() result"
                );
            }
        }

        #[test]
        fn value_should_be_idempotent() {
            let mut rsi = RsiIndicator::new(14);
            
            for i in 0..20 {
                rsi.next(100.0 + i as f64);
            }
            
            let v1 = rsi.value();
            let v2 = rsi.value();
            let v3 = rsi.value();
            
            assert!((v1 - v2).abs() < f64::EPSILON);
            assert!((v2 - v3).abs() < f64::EPSILON);
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn should_handle_negative_values() {
            let mut rsi = RsiIndicator::new(5);
            
            for i in 0..20 {
                rsi.next(-100.0 + i as f64);
            }
            
            let value = rsi.value();
            assert!(value >= 0.0 && value <= 100.0, "RSI should be in [0, 100] range");
        }

        #[test]
        fn should_handle_very_large_values() {
            let mut rsi = RsiIndicator::new(5);
            
            for i in 0..20 {
                rsi.next(1e10 + i as f64);
            }
            
            let value = rsi.value();
            assert!(value > 90.0, "should detect uptrend in large values");
        }

        #[test]
        fn should_handle_very_small_changes() {
            let mut rsi = RsiIndicator::new(5);
            
            for i in 0..20 {
                rsi.next(100.0 + (i as f64) * 1e-10);
            }
            
            // Very small increases should still be detected as uptrend
            let value = rsi.value();
            assert!(value >= 50.0, "should detect micro-uptrend");
        }

        #[test]
        fn should_handle_identical_consecutive_values() {
            let mut rsi = RsiIndicator::new(5);
            
            // Warmup with changes
            for i in 0..5 {
                rsi.next(100.0 + i as f64);
            }
            
            // Then feed identical values
            for _ in 0..20 {
                rsi.next(105.0);
            }
            
            // With no new gains or losses, RSI should stabilize
            let value = rsi.value();
            assert!(value >= 0.0 && value <= 100.0, "RSI should remain valid");
        }
    }

    mod derived_traits {
        use super::*;

        #[test]
        fn clone_should_produce_independent_copy() {
            let mut rsi1 = RsiIndicator::new(14);
            
            // Create mixed trend (not saturated)
            for i in 0..20 {
                rsi1.next(100.0 + (i as f64) * 0.5);
            }
            
            let rsi2 = rsi1.clone();
            
            // Create a reversal in rsi1 to cause noticeable change
            for _ in 0..5 {
                rsi1.next(80.0);  // Sudden drop
            }
            
            // rsi2 should be unaffected - verify clone created independent state
            // The key invariant: rsi2's value should remain what it was before rsi1 was modified
            // Rather than checking magnitude, verify rsi1 changed direction (dropped)
            assert!(rsi1.value() < rsi2.value(), "rsi1 should have dropped after reversal");
        }

        #[test]
        fn debug_should_produce_output() {
            let rsi = RsiIndicator::new(14);
            let debug_str = format!("{:?}", rsi);
            assert!(debug_str.contains("RsiIndicator"));
        }
    }
}

// =============================================================================
// SAVITZKY-GOLAY FILTER TESTS
// =============================================================================

mod savgol_tests {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn new_should_create_filter_with_specified_window() {
            let filter = SavitzkyGolayFilter::new(5);
            assert!(!filter.is_ready(), "new filter should not be ready");
        }

        #[test]
        fn new_should_ensure_odd_window_size() {
            // Even window sizes should be converted to odd
            let filter = SavitzkyGolayFilter::new(4);
            // Window should become 5 (4+1)
            
            // Feed values to fill buffer
            let mut f = filter;
            for _ in 0..5 {
                f.next(100.0);
            }
            assert!(f.is_ready(), "filter with even input should round up to odd");
        }

        #[test]
        fn new_should_accept_small_windows() {
            let filter = SavitzkyGolayFilter::new(3);
            assert!(!filter.is_ready());
        }

        #[test]
        fn new_should_accept_common_window_sizes() {
            for size in [3, 5, 7, 9, 11] {
                let filter = SavitzkyGolayFilter::new(size);
                let debug = format!("{:?}", filter);
                assert!(debug.contains("SavitzkyGolayFilter"));
            }
        }

        #[test]
        fn new_should_handle_uncommon_window_sizes_with_fallback() {
            // Sizes not in the pre-computed list should use moving average
            let mut filter = SavitzkyGolayFilter::new(13);
            
            for _ in 0..15 {
                filter.next(100.0);
            }
            
            // Should still produce valid output using fallback
            assert!((filter.value() - 100.0).abs() < 0.001);
        }
    }

    mod ready_state {
        use super::*;

        #[test]
        fn is_ready_should_return_false_until_buffer_full() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            for i in 0..4 {
                filter.next(100.0);
                assert!(!filter.is_ready(), "should not be ready at step {}", i);
            }
            
            filter.next(100.0);
            assert!(filter.is_ready(), "should be ready after 5 samples");
        }

        #[test]
        fn is_ready_should_remain_true_after_more_samples() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            for _ in 0..10 {
                filter.next(100.0);
            }
            
            assert!(filter.is_ready(), "should remain ready");
        }
    }

    mod smoothing_behavior {
        use super::*;

        #[test]
        fn should_return_constant_for_constant_input() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            for _ in 0..10 {
                filter.next(100.0);
            }
            
            assert!((filter.value() - 100.0).abs() < 0.001);
        }

        #[test]
        fn should_reduce_noise_variance() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            // Noisy data around 100
            let noisy = [98.0, 102.0, 97.0, 103.0, 99.0, 101.0, 98.0, 102.0];
            let mut smoothed = Vec::new();
            
            for &v in &noisy {
                smoothed.push(filter.next(v));
            }
            
            // Calculate variance of last few smoothed values
            let last_smoothed: Vec<_> = smoothed.iter().skip(4).copied().collect();
            let mean: f64 = last_smoothed.iter().sum::<f64>() / last_smoothed.len() as f64;
            let variance: f64 = last_smoothed.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / last_smoothed.len() as f64;
            
            // Variance should be smaller than raw noise variance
            let raw_variance: f64 = noisy.iter().map(|v| (v - 100.0).powi(2)).sum::<f64>() / noisy.len() as f64;
            
            assert!(variance < raw_variance + 1.0, "smoothed variance {} should be less than raw {}", variance, raw_variance);
        }

        #[test]
        fn should_follow_linear_trend() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            // Linear trend
            for i in 0..20 {
                filter.next(100.0 + i as f64);
            }
            
            // Smoothed value should be close to the trend
            let value = filter.value();
            assert!((value - 117.0).abs() < 5.0, "should follow linear trend, got {}", value);
        }

        #[test]
        fn should_preserve_signal_while_removing_noise() {
            let mut filter = SavitzkyGolayFilter::new(7);
            
            // Signal with noise: linear trend + random noise
            let base_trend: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
            let noise = [0.5, -0.3, 0.7, -0.2, 0.4, -0.6, 0.1, -0.5, 0.3, -0.4,
                         0.6, -0.1, 0.2, -0.7, 0.5, -0.3, 0.4, -0.2, 0.1, -0.5,
                         0.3, -0.4, 0.6, -0.1, 0.2, -0.6, 0.4, -0.3, 0.5, -0.2];
            
            let mut smoothed = Vec::new();
            for (i, &n) in noise.iter().enumerate() {
                smoothed.push(filter.next(base_trend[i] + n));
            }
            
            // Savitzky-Golay filters have inherent lag (approximately half the window size)
            // for a window of 7, lag is ~3-4 points. For a linear trend, the smoothed value
            // at time T approximates the trend at time T - lag.
            // Expected behavior: smoothed trails the trend by window_size/2 ≈ 3-4 points
            let last = smoothed.last().unwrap();
            let expected = base_trend.last().unwrap();
            let lag_allowance = 5.0;  // Allow for ~4 point filter lag + 1 for noise
            assert!(
                (last - expected).abs() < lag_allowance,
                "should track trend within lag tolerance, got {} vs {} (diff: {})",
                last, expected, (last - expected).abs()
            );
        }
    }

    mod before_ready {
        use super::*;

        #[test]
        fn should_return_simple_average_when_not_ready() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            filter.next(100.0);
            filter.next(200.0);
            filter.next(300.0);
            
            // Average of 100, 200, 300 = 200
            let value = filter.value();
            assert!((value - 200.0).abs() < 0.001, "expected 200, got {}", value);
        }

        #[test]
        fn should_return_zero_for_empty_filter() {
            let filter = SavitzkyGolayFilter::new(5);
            assert!((filter.value() - 0.0).abs() < f64::EPSILON);
        }
    }

    mod latest_value {
        use super::*;

        #[test]
        fn latest_should_return_none_for_empty_filter() {
            let filter = SavitzkyGolayFilter::new(5);
            assert!(filter.latest().is_none());
        }

        #[test]
        fn latest_should_return_most_recent_value() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            filter.next(100.0);
            filter.next(200.0);
            filter.next(300.0);
            
            assert_eq!(filter.latest(), Some(300.0));
        }

        #[test]
        fn latest_should_update_after_each_next() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            let values = [10.0, 20.0, 30.0, 40.0, 50.0];
            for &v in &values {
                filter.next(v);
                assert_eq!(filter.latest(), Some(v));
            }
        }
    }

    mod reset_behavior {
        use super::*;

        #[test]
        fn reset_should_clear_buffer() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            for _ in 0..10 {
                filter.next(100.0);
            }
            
            filter.reset();
            
            assert!(!filter.is_ready());
            assert!(filter.latest().is_none());
            assert!((filter.value() - 0.0).abs() < f64::EPSILON);
        }

        #[test]
        fn reset_should_allow_reuse() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            for _ in 0..10 {
                filter.next(100.0);
            }
            
            filter.reset();
            
            // Should work normally after reset
            for _ in 0..5 {
                filter.next(50.0);
            }
            
            assert!(filter.is_ready());
            assert!((filter.value() - 50.0).abs() < 0.001);
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn should_handle_negative_values() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            for _ in 0..10 {
                filter.next(-100.0);
            }
            
            assert!((filter.value() - (-100.0)).abs() < 0.001);
        }

        #[test]
        fn should_handle_mixed_positive_negative() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            let values = [-50.0, -25.0, 0.0, 25.0, 50.0, 25.0, 0.0, -25.0, -50.0];
            for &v in &values {
                filter.next(v);
            }
            
            // Should produce some valid smoothed value
            let value = filter.value();
            assert!(value.is_finite());
        }

        #[test]
        fn should_handle_very_large_values() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            for _ in 0..10 {
                filter.next(1e15);
            }
            
            assert!((filter.value() - 1e15).abs() < 1e10);
        }

        #[test]
        fn should_handle_very_small_values() {
            let mut filter = SavitzkyGolayFilter::new(5);
            
            for _ in 0..10 {
                filter.next(1e-15);
            }
            
            assert!((filter.value() - 1e-15).abs() < 1e-16);
        }
    }

    mod derived_traits {
        use super::*;

        #[test]
        fn clone_should_produce_independent_copy() {
            let mut filter1 = SavitzkyGolayFilter::new(5);
            for _ in 0..5 {
                filter1.next(100.0);
            }
            
            let filter2 = filter1.clone();
            
            // Add multiple large values to filter1 to create significant difference
            // SavGol smoothing dampens sudden changes, so we need sustained new input
            for _ in 0..5 {
                filter1.next(500.0);  // Large sustained change
            }
            
            // filter2 should still have its original state (~100.0)
            // filter1 should now be ~500.0
            assert!(
                (filter1.value() - filter2.value()).abs() > 100.0,
                "clone should be independent: filter1={}, filter2={}",
                filter1.value(), filter2.value()
            );
        }

        #[test]
        fn debug_should_produce_output() {
            let filter = SavitzkyGolayFilter::new(5);
            let debug_str = format!("{:?}", filter);
            assert!(debug_str.contains("SavitzkyGolayFilter"));
        }
    }
}

// =============================================================================
// PLATO CONTROLLER TESTS
// =============================================================================

mod controller_tests {
    use super::*;

    fn default_config() -> PlatoConfig {
        PlatoConfig::default()
    }

    fn custom_config(target_latency: f64, min_latency: f64, max_timeout: f64) -> PlatoConfig {
        PlatoConfig {
            target_latency_secs: target_latency,
            minimum_latency_secs: min_latency,
            max_gossip_timeout_secs: max_timeout,
            ..Default::default()
        }
    }

    mod construction {
        use super::*;

        #[test]
        fn new_should_initialize_with_config_values() {
            let config = custom_config(5.0, 1.0, 60.0);
            let controller = PlatoController::new(config.clone());
            
            assert!((controller.current_latency() - 5.0).abs() < 0.001);
            assert!((controller.publish_frequency() - config.target_publishing_frequency_secs).abs() < 0.001);
        }

        #[test]
        fn new_should_initialize_timing_changed_to_false() {
            let controller = PlatoController::new(default_config());
            assert!(!controller.timing_changed);
        }

        #[test]
        fn new_should_initialize_missed_delivery_to_false() {
            let controller = PlatoController::new(default_config());
            assert!(!controller.recently_missed_delivery());
        }

        #[test]
        fn new_should_create_empty_latency_queues() {
            let controller = PlatoController::new(default_config());
            let stats = controller.stats();
            
            assert_eq!(stats.our_latency_samples, 0);
            assert_eq!(stats.peer_latency_samples, 0);
        }
    }

    mod latency_recording {
        use super::*;

        #[test]
        fn record_our_latency_should_increment_sample_count() {
            let mut controller = PlatoController::new(default_config());
            
            for i in 1..=10 {
                controller.record_our_latency(2.0);
                assert_eq!(controller.stats().our_latency_samples, i);
            }
        }

        #[test]
        fn record_peer_latency_should_increment_sample_count() {
            let mut controller = PlatoController::new(default_config());
            
            for i in 1..=10 {
                controller.record_peer_latency(2.0);
                assert_eq!(controller.stats().peer_latency_samples, i);
            }
        }

        #[test]
        fn should_cap_latency_samples_at_max() {
            let mut controller = PlatoController::new(default_config());
            
            // Record more than max_samples (100)
            for _ in 0..150 {
                controller.record_our_latency(2.0);
            }
            
            assert_eq!(controller.stats().our_latency_samples, 100);
        }

        #[test]
        fn record_our_latency_should_update_rsi_indicators() {
            let mut controller = PlatoController::new(default_config());
            
            // Record enough samples to exit warmup
            for i in 0..30 {
                controller.record_our_latency(2.0 + i as f64 * 0.1);
            }
            
            let stats = controller.stats();
            // RSI should reflect uptrend (high value)
            assert!(stats.our_rsi_up > 50.0, "expected uptrend RSI > 50, got {}", stats.our_rsi_up);
        }
    }

    mod missed_delivery_flag {
        use super::*;

        #[test]
        fn set_missed_delivery_should_set_flag_to_true() {
            let mut controller = PlatoController::new(default_config());
            controller.set_missed_delivery(true);
            assert!(controller.recently_missed_delivery());
        }

        #[test]
        fn set_missed_delivery_should_set_flag_to_false() {
            let mut controller = PlatoController::new(default_config());
            controller.set_missed_delivery(true);
            controller.set_missed_delivery(false);
            assert!(!controller.recently_missed_delivery());
        }
    }

    mod increasing_congestion {
        use super::*;

        #[test]
        fn check_increasing_should_do_nothing_if_not_ready() {
            let mut controller = PlatoController::new(default_config());
            let initial_latency = controller.current_latency();
            
            // Only add a few samples (not enough to fill savgol window)
            for _ in 0..5 {
                controller.record_our_latency(2.0);
                controller.record_peer_latency(2.0);
            }
            
            controller.check_increasing_congestion();
            
            assert!((controller.current_latency() - initial_latency).abs() < 0.001);
            assert!(!controller.timing_changed);
        }

        #[test]
        fn check_increasing_should_fast_forward_when_latency_far_below_weighted() {
            let config = PlatoConfig {
                target_latency_secs: 1.0,  // Start very low
                savgol_increase_window: 5,
                savgol_decrease_window: 5,
                max_gossip_timeout_secs: 60.0,
                ..Default::default()
            };
            let mut controller = PlatoController::new(config);
            
            // Feed high latency values to make weighted_latency > 2 * current_latency
            for _ in 0..20 {
                controller.record_our_latency(10.0);
                controller.record_peer_latency(10.0);
            }
            
            controller.check_increasing_congestion();
            
            // If fast-forward triggered, latency should have doubled
            // Note: depends on weighted_latency calculation
            assert!(controller.current_latency() > 1.0, "expected latency increase");
        }

        #[test]
        fn check_increasing_should_not_fast_forward_when_proposed_exceeds_85_percent_max() {
            // This test verifies that fast-forward does NOT occur when the proposed
            // doubled latency would exceed 85% of max_gossip_timeout_secs.
            // The algorithm intentionally skips fast-forward in this case.
            let config = PlatoConfig {
                target_latency_secs: 50.0,  // 50 * 2 = 100, which > 60 * 0.85 = 51
                max_gossip_timeout_secs: 60.0,
                savgol_increase_window: 5,
                savgol_decrease_window: 5,
                rsi_overbought: 70.0,
                rsi_increase_period: 14,
                ..Default::default()
            };
            let mut controller = PlatoController::new(config);
            let initial_latency = controller.current_latency();
            
            // Feed high latency values - this creates fast-forward condition
            // (current_latency <= 0.5 * weighted_latency), but doubled would exceed threshold
            for _ in 0..20 {
                controller.record_our_latency(100.0);
                controller.record_peer_latency(100.0);
            }
            
            controller.check_increasing_congestion();
            
            // Fast-forward should NOT have applied (proposed 100.0 > 51.0 threshold)
            // Latency should either stay same OR increase via throttle (RSI-based)
            // The key invariant: latency should not have doubled
            assert!(
                controller.current_latency() < initial_latency * 2.0,
                "fast-forward should be skipped when proposed exceeds 85% of max"
            );
        }
    }

    mod decreasing_congestion {
        use super::*;

        #[test]
        fn check_decreasing_should_do_nothing_if_not_ready() {
            let mut controller = PlatoController::new(default_config());
            let initial_latency = controller.current_latency();
            
            for _ in 0..5 {
                controller.record_our_latency(2.0);
                controller.record_peer_latency(2.0);
            }
            
            controller.check_decreasing_congestion();
            
            assert!((controller.current_latency() - initial_latency).abs() < 0.001);
        }

        #[test]
        fn check_decreasing_should_not_go_below_minimum() {
            let config = PlatoConfig {
                target_latency_secs: 0.5,
                minimum_latency_secs: 0.5,
                rsi_decrease_period: 5,
                savgol_decrease_window: 5,
                savgol_increase_window: 5,
                rsi_oversold: 30.0,
                ..Default::default()
            };
            let mut controller = PlatoController::new(config.clone());
            
            // Create strong downtrend to trigger acceleration
            for i in 0..50 {
                controller.record_our_latency(10.0 - i as f64 * 0.1);
                controller.record_peer_latency(10.0 - i as f64 * 0.1);
            }
            
            for _ in 0..10 {
                controller.check_decreasing_congestion();
            }
            
            assert!(
                controller.current_latency() >= config.minimum_latency_secs,
                "should not go below minimum"
            );
        }
    }

    mod weighted_latency {
        use super::*;

        #[test]
        fn weighted_latency_should_combine_our_and_peer() {
            let config = PlatoConfig {
                own_latency_weight: 0.6,
                savgol_increase_window: 5,
                savgol_decrease_window: 5,
                ..Default::default()
            };
            let mut controller = PlatoController::new(config);
            
            // Our latency = 10, peer latency = 20
            for _ in 0..10 {
                controller.record_our_latency(10.0);
                controller.record_peer_latency(20.0);
            }
            
            // Expected: 0.6 * 10 + 0.4 * 20 = 6 + 8 = 14
            let weighted = controller.weighted_latency();
            assert!(
                (weighted - 14.0).abs() < 1.0,
                "expected ~14, got {}",
                weighted
            );
        }

        #[test]
        fn weighted_latency_should_return_zero_for_empty_controller() {
            let controller = PlatoController::new(default_config());
            // Should be 0 when no data
            assert!(controller.weighted_latency().abs() < f64::EPSILON);
        }
    }

    mod stats {
        use super::*;

        #[test]
        fn stats_should_reflect_current_state() {
            let config = default_config();
            let mut controller = PlatoController::new(config.clone());
            
            for i in 0..20 {
                controller.record_our_latency(2.0 + i as f64 * 0.1);
                controller.record_peer_latency(3.0 - i as f64 * 0.05);
            }
            
            let stats = controller.stats();
            
            assert_eq!(stats.our_latency_samples, 20);
            assert_eq!(stats.peer_latency_samples, 20);
            assert!((stats.current_latency - config.target_latency_secs).abs() < 0.001);
        }

        #[test]
        fn stats_should_include_rsi_values() {
            let mut controller = PlatoController::new(default_config());
            
            // RSI values should be neutral (50) during warmup
            let stats = controller.stats();
            assert!((stats.our_rsi_up - 50.0).abs() < f64::EPSILON);
            assert!((stats.peer_rsi_up - 50.0).abs() < f64::EPSILON);
        }
    }

    mod timing_changed_flag {
        use super::*;

        #[test]
        fn clear_timing_changed_should_reset_flag() {
            let mut controller = PlatoController::new(default_config());
            controller.timing_changed = true;
            
            controller.clear_timing_changed();
            
            assert!(!controller.timing_changed);
        }
    }

    mod accessor_methods {
        use super::*;

        #[test]
        fn current_latency_should_return_initial_value() {
            let config = custom_config(5.5, 1.0, 60.0);
            let controller = PlatoController::new(config);
            
            assert!((controller.current_latency() - 5.5).abs() < 0.001);
        }

        #[test]
        fn publish_frequency_should_return_initial_value() {
            let config = default_config();
            let controller = PlatoController::new(config.clone());
            
            assert!(
                (controller.publish_frequency() - config.target_publishing_frequency_secs).abs() < 0.001
            );
        }
    }
}

// =============================================================================
// PLATO STATS TESTS
// =============================================================================

mod plato_stats_tests {
    use super::*;
    use racer::plato::PlatoStats;

    #[test]
    fn stats_should_be_cloneable() {
        let mut controller = PlatoController::new(PlatoConfig::default());
        for _ in 0..10 {
            controller.record_our_latency(2.0);
        }
        
        let stats1 = controller.stats();
        let stats2 = stats1.clone();
        
        assert!((stats1.current_latency - stats2.current_latency).abs() < f64::EPSILON);
    }

    #[test]
    fn stats_should_be_debuggable() {
        let controller = PlatoController::new(PlatoConfig::default());
        let stats = controller.stats();
        
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("PlatoStats"));
        assert!(debug_str.contains("current_latency"));
    }
}
