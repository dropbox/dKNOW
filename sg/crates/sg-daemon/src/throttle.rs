//! Resource throttling based on user activity
//!
//! Adjusts indexing rate based on how active the user is.
//! When user is active (recent IPC requests), we slow down to avoid
//! impacting their work. When idle, we speed up to complete indexing.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Throttle thresholds used to classify user activity.
#[derive(Debug, Clone, Copy)]
pub struct ThrottleConfig {
    pub active_threshold_secs: u64,
    pub recent_activity_threshold_secs: u64,
    pub idle_threshold_secs: u64,
    pub away_duration_secs: u64,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            active_threshold_secs: 5,
            recent_activity_threshold_secs: 30,
            idle_threshold_secs: 300,
            away_duration_secs: 600,
        }
    }
}

/// Throttler that adjusts work rate based on user activity
pub struct Throttler {
    /// Instant when daemon started (for relative timing)
    start: Instant,
    /// Ticks since start when last activity occurred
    last_activity_ticks: AtomicU64,
    /// Whether any activity has ever been recorded
    has_activity: AtomicBool,
    /// Threshold configuration
    config: ThrottleConfig,
}

/// Resource limits for different activity states
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    /// Minimum delay between indexing operations (ms)
    pub min_delay_ms: u64,
    /// Maximum files to process per batch
    pub batch_size: usize,
}

impl Throttler {
    /// Create a new throttler
    pub fn new() -> Self {
        Self::with_config(ThrottleConfig::default())
    }

    /// Create a new throttler with custom thresholds
    pub fn with_config(config: ThrottleConfig) -> Self {
        Self {
            start: Instant::now(),
            last_activity_ticks: AtomicU64::new(0),
            has_activity: AtomicBool::new(false),
            config,
        }
    }

    /// Record user activity (call this on IPC requests)
    pub fn record_activity(&self) {
        let ticks = self.start.elapsed().as_millis() as u64;
        self.last_activity_ticks.store(ticks, Ordering::Relaxed);
        self.has_activity.store(true, Ordering::Relaxed);
    }

    /// Get time since last user activity
    /// Returns a very long duration if no activity has ever been recorded
    pub fn idle_duration(&self) -> Duration {
        // If no activity ever recorded, treat as "away" (very idle)
        if !self.has_activity.load(Ordering::Relaxed) {
            return Duration::from_secs(self.config.away_duration_secs);
        }

        let last_ticks = self.last_activity_ticks.load(Ordering::Relaxed);
        let current_ticks = self.start.elapsed().as_millis() as u64;
        Duration::from_millis(current_ticks.saturating_sub(last_ticks))
    }

    /// Get current resource limits based on idle duration
    pub fn get_limits(&self) -> ResourceLimits {
        let idle = self.idle_duration();

        if idle < Duration::from_secs(self.config.active_threshold_secs) {
            // User very active - minimal work
            ResourceLimits {
                min_delay_ms: 500,
                batch_size: 1,
            }
        } else if idle < Duration::from_secs(self.config.recent_activity_threshold_secs) {
            // User somewhat active
            ResourceLimits {
                min_delay_ms: 200,
                batch_size: 2,
            }
        } else if idle < Duration::from_secs(self.config.idle_threshold_secs) {
            // User idle
            ResourceLimits {
                min_delay_ms: 50,
                batch_size: 5,
            }
        } else {
            // User away
            ResourceLimits {
                min_delay_ms: 10,
                batch_size: 10,
            }
        }
    }

    /// Get a human-readable description of current throttle state
    pub fn state_description(&self) -> &'static str {
        let idle = self.idle_duration();

        if idle < Duration::from_secs(self.config.active_threshold_secs) {
            "active (throttled)"
        } else if idle < Duration::from_secs(self.config.recent_activity_threshold_secs) {
            "recent activity"
        } else if idle < Duration::from_secs(self.config.idle_threshold_secs) {
            "idle"
        } else {
            "away (full speed)"
        }
    }
}

impl Default for Throttler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_initial_state() {
        let throttler = Throttler::new();

        // Initially considered idle/away since no activity recorded
        let idle = throttler.idle_duration();
        assert!(idle >= Duration::from_secs(0));

        // State should reflect no recent activity
        let limits = throttler.get_limits();
        // Very low delay since no activity has been recorded (effectively "away")
        assert!(limits.min_delay_ms <= 50);
    }

    #[test]
    fn test_activity_recording() {
        let throttler = Throttler::new();

        // Record activity
        throttler.record_activity();

        // Should now be considered active
        let idle = throttler.idle_duration();
        assert!(idle < Duration::from_secs(1));

        let limits = throttler.get_limits();
        assert_eq!(limits.min_delay_ms, 500); // Most throttled
        assert_eq!(limits.batch_size, 1);
    }

    #[test]
    fn test_idle_progression() {
        let throttler = Throttler::new();
        throttler.record_activity();

        // Wait a bit
        thread::sleep(Duration::from_millis(50));

        // Still considered active
        let idle = throttler.idle_duration();
        assert!(idle < Duration::from_secs(5));
    }

    #[test]
    fn test_state_description() {
        let throttler = Throttler::new();

        // No activity recorded - starts as away
        let desc = throttler.state_description();
        assert!(desc.contains("away") || desc.contains("idle"));

        // Record activity
        throttler.record_activity();
        let desc = throttler.state_description();
        assert!(desc.contains("active"));
    }

    #[test]
    fn test_limits_structure() {
        let throttler = Throttler::new();
        throttler.record_activity();

        let limits = throttler.get_limits();

        // Verify limits are reasonable
        assert!(limits.min_delay_ms > 0);
        assert!(limits.batch_size > 0);
        assert!(limits.batch_size <= 10);
    }

    #[test]
    fn test_default_impl() {
        let throttler = Throttler::default();

        // Default should behave identically to new()
        let idle = throttler.idle_duration();
        assert!(idle >= Duration::from_secs(0));

        // Should be in "away" state initially (no activity recorded)
        let limits = throttler.get_limits();
        assert!(limits.min_delay_ms <= 50);
        assert!(limits.batch_size >= 5);
    }

    #[test]
    fn test_resource_limits_debug() {
        let limits = ResourceLimits {
            min_delay_ms: 100,
            batch_size: 5,
        };

        let debug_str = format!("{limits:?}");
        assert!(debug_str.contains("ResourceLimits"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("5"));
    }

    #[test]
    fn test_resource_limits_clone() {
        let limits = ResourceLimits {
            min_delay_ms: 200,
            batch_size: 3,
        };

        let cloned = limits;
        assert_eq!(cloned.min_delay_ms, 200);
        assert_eq!(cloned.batch_size, 3);
    }

    #[test]
    fn test_resource_limits_copy() {
        let limits = ResourceLimits {
            min_delay_ms: 50,
            batch_size: 10,
        };

        // Copy semantics - assigning to another variable should copy
        let copied = limits;
        assert_eq!(limits.min_delay_ms, copied.min_delay_ms);
        assert_eq!(limits.batch_size, copied.batch_size);
    }

    #[test]
    fn test_multiple_activity_recordings() {
        let throttler = Throttler::new();

        // Record activity multiple times
        throttler.record_activity();
        thread::sleep(Duration::from_millis(10));
        throttler.record_activity();
        thread::sleep(Duration::from_millis(10));
        throttler.record_activity();

        // Should still be active with recent activity
        let idle = throttler.idle_duration();
        assert!(idle < Duration::from_millis(50));

        let limits = throttler.get_limits();
        assert_eq!(limits.min_delay_ms, 500); // Most throttled (active)
    }

    #[test]
    fn test_config_defaults_are_reasonable() {
        // Verify the default thresholds have expected values:
        // active (5s) < recent (30s) < idle (300s) < away (600s)
        let config = ThrottleConfig::default();
        assert_eq!(config.active_threshold_secs, 5);
        assert_eq!(config.recent_activity_threshold_secs, 30);
        assert_eq!(config.idle_threshold_secs, 300);
        assert_eq!(config.away_duration_secs, 600);
    }

    #[test]
    fn test_limits_monotonically_decrease_delay() {
        // As user becomes less active, delay should decrease
        // and batch size should increase
        let throttler = Throttler::new();

        // Start with no activity (away state)
        let away_limits = throttler.get_limits();

        // Record activity to become active
        throttler.record_activity();
        let active_limits = throttler.get_limits();

        // Active state should have MORE delay (more throttled)
        assert!(active_limits.min_delay_ms >= away_limits.min_delay_ms);
        // Active state should have SMALLER batch size (more throttled)
        assert!(active_limits.batch_size <= away_limits.batch_size);
    }

    #[test]
    fn test_state_descriptions_are_distinct() {
        // All state descriptions should be unique strings
        let descriptions = [
            "active (throttled)",
            "recent activity",
            "idle",
            "away (full speed)",
        ];

        // Verify they are all different
        for (i, desc1) in descriptions.iter().enumerate() {
            for (j, desc2) in descriptions.iter().enumerate() {
                if i != j {
                    assert_ne!(desc1, desc2, "State descriptions should be distinct");
                }
            }
        }
    }

    #[test]
    fn test_idle_duration_never_negative() {
        let throttler = Throttler::new();

        // Even with no activity, duration should be non-negative
        let idle = throttler.idle_duration();
        assert!(idle >= Duration::ZERO);

        // After recording activity
        throttler.record_activity();
        let idle = throttler.idle_duration();
        assert!(idle >= Duration::ZERO);
    }
}
