/// HLC error type.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum HlcError {
    /// Timestamp is out of range.
    #[error("Out of range timestamp")]
    OutOfRangeTimestamp,

    /// Drift is too large.
    #[error("Drift exeeded the maximum allowed: {0} > {1}")]
    DriftTooLarge(usize, usize),
}

/// HLC result type.
pub type HlcResult<T> = Result<T, HlcError>;
