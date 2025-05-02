use {crate::epoch::CUSTOM_EPOCH, chrono::Utc, parking_lot::RwLock};

/// Provides current time.
pub trait ClockSource: Default {
    /// The current timestamp in milliseconds since the Unix epoch.
    fn current_timestamp(&self) -> i64;
}

/// UTC clock.
///
/// Granularity is in milliseconds.
#[derive(Default)]
pub struct UtcClock;

impl ClockSource for UtcClock {
    fn current_timestamp(&self) -> i64 {
        Utc::now().timestamp_millis()
    }
}

/// Manual clock.
///
/// Useful for testing purposes.
pub struct ManualClock {
    /// The current timestamp in milliseconds since the Unix epoch.
    timestamp: RwLock<i64>,
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new(CUSTOM_EPOCH)
    }
}

impl ClockSource for ManualClock {
    fn current_timestamp(&self) -> i64 {
        let r = self.timestamp.read();
        *r
    }
}

impl ManualClock {
    /// Creates new clock.
    pub fn new(timestamp: i64) -> Self {
        Self {
            timestamp: RwLock::new(timestamp),
        }
    }

    /// Sets the current timestamp.
    pub fn set_current_timestamp(&self, timestamp: i64) {
        let mut w = self.timestamp.write();
        *w = timestamp;
    }
}
