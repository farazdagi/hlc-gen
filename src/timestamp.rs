use {
    crate::{
        epoch::{CUSTOM_EPOCH, CustomEpochTimestamp},
        error::{HlcError, HlcResult},
    },
    chrono::Utc,
    std::sync::atomic::{AtomicU64, Ordering},
};

/// Number of bits to represent physical time in milliseconds since custom
/// epoch.
static PT_BITS: u8 = 42;

/// Maximum value for physical time.
static PT_MAX: u64 = (1 << PT_BITS) - 1;

/// Number of bits to represent logical clock counter.
static LC_BITS: u8 = 22;

/// Maximum value for logical clock.
static LC_MAX: u64 = (1 << LC_BITS) - 1;

/// Hybrid logical clock (HLC) timestamp.
///
/// This is a lock-free implementation of a hybrid logical clock (HLC)
/// timestamp.
///
/// The timestamp is represented as a 64-bit unsigned integer. The upper 42 bits
/// represent the physical time in milliseconds since a custom epoch, and the
/// lower 22 bits represent the logical clock count.
///
/// Normally, you don't need to worry about the details of the representation.
///
/// Whenever you need to create a new timestamp, use the
/// [`new()`](Self::new()) to create a timestamp with the current time,
/// or [`from_parts()`](Self::from_parts()) to create a timestamp with a
/// specific Unix timestamp (in ms) and logical clock count.
///
/// When you need to update the timestamp, use the [`update()`](Self::update())
/// method.
///
/// Finally, you can use the [`as_u64()`](Self::as_u64()) method to get the raw
/// data, which is guaranteed to be monotonically increasing and capturing the
/// happens-before relationship.
///
/// To get the physical time and logical clock count, use the
/// [`parts()`](Self::parts()) which returns a tuple of `(pt, lc)`.
#[derive(Debug)]
pub struct HlcTimestamp(AtomicU64);

impl Default for HlcTimestamp {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<u64> for HlcTimestamp {
    type Error = HlcError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let pt = (value >> LC_BITS) & PT_MAX;
        let lc = value & LC_MAX;
        Self::from_parts(CustomEpochTimestamp::to_unix_timestamp(pt), lc)
    }
}

impl Clone for HlcTimestamp {
    fn clone(&self) -> Self {
        let current_value = self.0.load(Ordering::Acquire);
        // Since there are no possible way to create invalid HLC timestamp (internal
        // data is not exposed and update method returns error on invalid values), this
        // conversion is infallible.
        current_value
            .try_into()
            .expect("Failed to clone HLC timestamp")
    }
}

impl PartialEq for HlcTimestamp {
    fn eq(&self, other: &Self) -> bool {
        self.as_u64() == other.as_u64()
    }
}

impl Eq for HlcTimestamp {}

impl PartialOrd for HlcTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.as_u64().cmp(&other.as_u64()))
    }
}

impl Ord for HlcTimestamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_u64().cmp(&other.as_u64())
    }
}

impl HlcTimestamp {
    /// Creates a new HLC timestamp.
    pub fn new() -> Self {
        let unix_timestamp = Utc::now().timestamp_millis();
        let epoch_timestamp = (unix_timestamp - CUSTOM_EPOCH) as u64;
        Self(AtomicU64::new(epoch_timestamp << LC_BITS))
    }

    /// Creates a new HLC timestamp from the given physical time and logical
    /// clock count.
    pub fn from_parts(pt: i64, lc: u64) -> HlcResult<Self> {
        if pt > PT_MAX as i64 {
            return Err(HlcError::PhysicalTimeExceedsMax(pt, PT_MAX));
        }
        if lc > LC_MAX {
            return Err(HlcError::LogicalClockExceedsMax(lc, LC_MAX));
        }

        // Convert the physical time to milliseconds since the custom epoch.
        let ts = CustomEpochTimestamp::from_unix_timestamp(pt)?;

        let combined = (ts.millis() << LC_BITS) | lc;
        Ok(Self(AtomicU64::new(combined)))
    }

    /// Sets the physical time and logical clock count.
    ///
    /// Expected closure gets the current physical time and logical clock count
    /// at the moment of the call and must return the new values for both.
    ///
    /// This is an atomic operation that ensures thread safety in a lock-free
    /// fashion. Either both values are updated or none are.
    pub fn update<F>(&self, new_values: F) -> HlcResult<HlcTimestamp>
    where
        F: Fn(i64, u64) -> HlcResult<(i64, u64)>,
    {
        loop {
            let current = self.0.load(Ordering::Acquire);

            // Obtain new values for physical time and logical clock count.
            let (pt, lc) = new_values(
                CustomEpochTimestamp::to_unix_timestamp((current >> LC_BITS) & PT_MAX),
                current & LC_MAX,
            )?;

            if pt > PT_MAX as i64 {
                return Err(HlcError::PhysicalTimeExceedsMax(pt, PT_MAX));
            }
            if lc > LC_MAX {
                return Err(HlcError::LogicalClockExceedsMax(lc, LC_MAX));
            }

            let ts = CustomEpochTimestamp::from_unix_timestamp(pt)?;
            let new_combined = (ts.millis() << LC_BITS) | lc;

            if self
                .0
                .compare_exchange(current, new_combined, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(HlcTimestamp(AtomicU64::new(new_combined)));
            }
        }
    }

    /// Returns the current HLC timestamp as a number.
    pub fn as_u64(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }

    /// Creates a new HLC timestamp from the given u64 value.
    ///
    /// The encoded value should be in the format of the HLC timestamp.
    pub fn from_u64(value: u64) -> HlcResult<Self> {
        value.try_into()
    }

    /// Returns the current physical timestamp and logical clock count as a
    /// tuple.
    ///
    /// This operation is atomic, as it uses a single load operation to get the
    /// inner value.
    pub fn parts(&self) -> (i64, u64) {
        let raw_value = self.as_u64();
        let pt = (raw_value >> LC_BITS) & PT_MAX;
        let lc = raw_value & LC_MAX;
        (CustomEpochTimestamp::to_unix_timestamp(pt), lc)
    }
}
