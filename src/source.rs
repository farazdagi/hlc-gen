use {crate::epoch::CUSTOM_EPOCH, chrono::Utc, parking_lot::RwLock};

/// Provides the current timestamp in milliseconds since the Unix epoch.
pub trait TimestampSource: Default {
    /// Returns the current timestamp in milliseconds since the Unix epoch.
    fn current_timestamp(&self) -> i64;
}

/// Implementation of the `CurrentTimestamp` trait using UTC.
#[derive(Default)]
pub struct UtcTimestamp;

impl TimestampSource for UtcTimestamp {
    fn current_timestamp(&self) -> i64 {
        Utc::now().timestamp_millis()
    }
}

/// Implementation of the `CurrentTimestamp` trait using a manual timestamp.
///
/// Useful for testing purposes.
pub struct ManualTimestamp {
    /// The current timestamp in milliseconds since the Unix epoch.
    timestamp: RwLock<i64>,
}

impl Default for ManualTimestamp {
    fn default() -> Self {
        Self::new(CUSTOM_EPOCH)
    }
}

impl TimestampSource for ManualTimestamp {
    fn current_timestamp(&self) -> i64 {
        let r = self.timestamp.read();
        *r
    }
}

impl ManualTimestamp {
    /// Creates a new `ManualTimestamp` with the specified timestamp.
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
