use {
    crate::{
        epoch::CustomEpochTimestamp,
        error::{HlcError, HlcResult},
    },
    std::{
        ops::{Add, AddAssign, Sub, SubAssign},
        sync::atomic::{AtomicU64, Ordering},
    },
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
/// This is a wrapper around raw `u64` data of HLC atomic timestamp.
///
/// The timestamp is represented as a 64-bit unsigned integer. The upper 42 bits
/// represent the physical time in milliseconds since a custom epoch, and the
/// lower 22 bits represent the logical clock count.
///
/// Normally, you don't need to worry about the details of the representation.
///
/// Whenever you need to create a new timestamp, use the
/// [`new()`](Self::new()) to create a timestamp with the given time,
/// or [`from_parts()`](Self::from_parts()) to create a timestamp with a
/// specific Unix timestamp (in ms) and logical clock count.
///
/// To get the physical time and logical clock count, use the
/// [`parts()`](Self::parts()) which returns a tuple of `(pt, lc)`.
///
/// Alternatively, rely on [`timestamp()`](Self::timestamp()) and
/// [`count()`](Self::count()) methods to get the physical time and logical
/// clock count.
///
/// Finally, you can use the [`as_u64()`](Self::as_u64()) method to get the raw
/// data, which is guaranteed to be monotonically increasing and capturing the
/// happens-before relationship.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HlcTimestamp(u64);

impl std::fmt::Display for HlcTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Customize the output format here
        write!(
            f,
            "HlcTimestamp {{ timestamp: {}, count: {} }}",
            self.timestamp(), self.count()
        )
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

macro_rules! impl_sub {
    ($lhs:ty, $rhs:ty) => {
        impl Sub<$rhs> for $lhs {
            type Output = i64;

            fn sub(self, rhs: $rhs) -> Self::Output {
                let pt1 = ((self.0 >> LC_BITS) & PT_MAX) as i64;
                let pt2 = ((rhs.0 >> LC_BITS) & PT_MAX) as i64;
                pt1 - pt2
            }
        }
    };
}

impl_sub!(HlcTimestamp, HlcTimestamp);
impl_sub!(&HlcTimestamp, &HlcTimestamp);
impl_sub!(HlcTimestamp, &HlcTimestamp);
impl_sub!(&HlcTimestamp, HlcTimestamp);

impl Sub<u64> for HlcTimestamp {
    type Output = Self;

    fn sub(self, ts: u64) -> Self::Output {
        let (pt, lc) = self.split();
        HlcTimestamp((pt.wrapping_sub(ts) << LC_BITS) | lc)
    }
}

impl SubAssign<u64> for HlcTimestamp {
    fn sub_assign(&mut self, ts: u64) {
        let (pt, lc) = self.split();
        self.0 = (pt.wrapping_sub(ts) << LC_BITS) | lc;
    }
}

impl Add<u64> for HlcTimestamp {
    type Output = Self;

    fn add(self, ts: u64) -> Self::Output {
        let (pt, lc) = self.split();
        HlcTimestamp((pt.wrapping_add(ts) << LC_BITS) | lc)
    }
}

impl AddAssign<u64> for HlcTimestamp {
    fn add_assign(&mut self, ts: u64) {
        let (pt, lc) = self.split();
        self.0 = (pt.wrapping_add(ts) << LC_BITS) | lc;
    }
}

impl HlcTimestamp {
    /// Creates a new HLC timestamp from incoming physical time.
    pub fn new(unix_timestamp: i64) -> HlcResult<Self> {
        Self::from_parts(unix_timestamp, 0)
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
        Ok(Self(combined))
    }

    /// Unix timestamp in milliseconds.
    pub fn timestamp(&self) -> i64 {
        CustomEpochTimestamp::to_unix_timestamp((self.0 >> LC_BITS) & PT_MAX)
    }

    /// Logical clock count.
    pub fn count(&self) -> u64 {
        self.0 & LC_MAX
    }

    /// Returns the physical time and logical clock count as a tuple.
    pub fn parts(&self) -> (i64, u64) {
        (self.timestamp(), self.count())
    }

    /// Returns the raw `u64` value of the HLC ID.
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Returns *raw* physical time and logical clock count parts.
    fn split(&self) -> (u64, u64) {
        let pt = (self.0 >> LC_BITS) & PT_MAX;
        let lc = self.0 & LC_MAX;
        (pt, lc)
    }
}

#[derive(Debug)]
pub(crate) struct HlcAtomicTimestamp(AtomicU64);

impl From<HlcTimestamp> for HlcAtomicTimestamp {
    fn from(ts: HlcTimestamp) -> Self {
        HlcAtomicTimestamp(AtomicU64::new(ts.0))
    }
}

impl HlcAtomicTimestamp {
    /// Sets the physical time and logical clock count.
    ///
    /// Expected closure gets the current physical time and logical clock count
    /// at the moment of the call and must return the new values for both.
    ///
    /// This is an atomic operation that ensures thread safety in a lock-free
    /// fashion. Either both values are updated or none are.
    pub fn update<F>(&self, new_values: F) -> HlcResult<HlcAtomicTimestamp>
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
                return Ok(HlcAtomicTimestamp(AtomicU64::new(new_combined)));
            }
        }
    }

    /// Creates a new HLC timestamp snapshot.
    pub fn snapshot(&self) -> HlcTimestamp {
        HlcTimestamp(self.0.load(Ordering::Acquire))
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::epoch::EPOCH, chrono::Utc, std::sync::Arc};

    #[test]
    fn concurrent_updates_to_atomic_timestamp() {
        let timestamp = Arc::new(HlcAtomicTimestamp(AtomicU64::new(0)));

        // Create multiple threads to update the timestamp concurrently.
        let mut handles = vec![];
        for t in 0..10 {
            let timestamp_clone = Arc::clone(&timestamp);
            handles.push(std::thread::spawn(move || {
                for i in 0..100 {
                    let _ =
                        timestamp_clone.update(move |_pt, _lc| Ok((EPOCH + t * 100 + i, 67890)));
                }
            }));
        }
        // Wait for all threads to finish.
        for handle in handles {
            handle.join().unwrap();
        }

        // Check that the timestamp is updated correctly.
        let final_timestamp = timestamp.snapshot().timestamp();

        assert!(final_timestamp >= EPOCH);
        assert!(final_timestamp <= EPOCH + 1000);

        // One of the threads made the last update, so possible values are in range
        // [EPOCH, EPOCH + 1000]
        assert!((final_timestamp + 1) % 100 == 0);
    }

    #[test]
    fn arithmetics() {
        let start = Utc::now().timestamp_millis();
        let t1 = HlcTimestamp::from_parts(start, 123).unwrap();

        let t2 = t1 + 1000;
        assert_eq!(t2.timestamp(), start + 1000);
        assert_eq!(t2.count(), 123);

        let mut t3 = t2 - 1000;
        assert_eq!(t3, t1);
        assert_eq!(t3.timestamp(), start);
        assert_eq!(t3.count(), 123);

        t3 += 1000;
        assert_eq!(t3.timestamp(), start + 1000);
        assert_eq!(t3.count(), 123);

        t3 -= 1000;
        assert_eq!(t3, t1);
        assert_eq!(t3.timestamp(), start);

        assert_eq!(t2 - t1, 1000i64);
        assert_eq!(t1 - t2, -1000i64);

        assert_eq!(&t2 - &t1, 1000i64);
        assert_eq!(&t1 - &t2, -1000i64);

        assert_eq!(&t2 - t1, 1000i64);
        assert_eq!(&t1 - t2, -1000i64);

        assert_eq!(t2 - &t1, 1000i64);
        assert_eq!(t1 - &t2, -1000i64);
    }
}
