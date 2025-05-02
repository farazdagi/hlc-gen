use crate::error::{HlcError, HlcResult};

/// Pre-calculated custom epoch.
///
/// 2024-01-01 00:00:00 UTC in milliseconds since Unix epoch
pub const EPOCH: i64 = 1_704_067_200_000;

/// Timestamps in milliseconds since a custom epoch (2024-01-01 00:00:00 UTC).
#[derive(Debug)]
pub struct CustomEpochTimestamp(u64);

impl CustomEpochTimestamp {
    /// Creates a new `CustomEpochTimestamp` from the given milliseconds since
    /// the custom epoch.
    pub fn from_millis(ms: u64) -> Self {
        Self(ms)
    }

    /// Returns the stored timestamp in milliseconds since the custom epoch.
    pub fn millis(&self) -> u64 {
        self.0
    }

    /// Creates a new `CustomEpochTimestamp` from the given Unix timestamp in
    /// milliseconds.
    pub fn from_unix_timestamp(unix_timestamp: i64) -> HlcResult<Self> {
        if unix_timestamp < EPOCH {
            return Err(HlcError::TimestampBelowMin(unix_timestamp, EPOCH));
        }
        Ok(Self::from_millis((unix_timestamp - EPOCH) as u64))
    }

    /// Returns the timestamp in milliseconds since the Unix epoch for a given
    /// number of milliseconds since the custom epoch.
    pub fn to_unix_timestamp(ms: u64) -> i64 {
        ms as i64 + EPOCH
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        chrono::{TimeZone, Utc},
    };

    #[test]
    fn custom_epoch_is_correct() {
        // Calculate the expected epoch using chrono
        let expected_epoch = Utc
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .unwrap()
            .timestamp_millis();

        // Assert that the pre-calculated epoch matches the expected value
        assert_eq!(EPOCH, expected_epoch);
    }

    #[test]
    fn conversion_to_and_from_unix_timestamp() {
        let unix_ts = 1704067200123; // 2024-01-01 00:00:00.123 UTC
        let custom_ts = CustomEpochTimestamp::from_unix_timestamp(unix_ts).unwrap();

        // Check milliseconds from custom epoch
        assert_eq!(custom_ts.millis(), 123);

        // Convert back to Unix timestamp
        let back_to_unix = CustomEpochTimestamp::to_unix_timestamp(custom_ts.millis());
        assert_eq!(back_to_unix, unix_ts);
    }
}
