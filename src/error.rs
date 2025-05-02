/// HLC error type.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum HlcError {
    /// Timestamp is out of range.
    #[error("Out of range timestamp")]
    OutOfRangeTimestamp,

    /// Drift is too large.
    #[error("Drift exeeded the maximum allowed: {0} > {1}")]
    DriftTooLarge(usize, usize),

    /// Physical time exceeds maximum value.
    #[error("Physical time exceeds maximum value: {0} > {1}")]
    PhysicalTimeExceedsMax(i64, u64),

    /// Logical clock exceeds maximum value.
    #[error("Logical clock exceeds maximum value: {0} > {1}")]
    LogicalClockExceedsMax(u64, u64),

    /// Timestamp is below the minimum value.
    #[error("Timestamp is below the minimum value: {0} < {1}")]
    TimestampBelowMin(i64, i64),
}

/// HLC result type.
pub type HlcResult<T> = Result<T, HlcError>;
