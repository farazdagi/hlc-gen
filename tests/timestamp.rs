mod common;

use {chrono::Utc, common::EPOCH, hlc_gen::HlcTimestamp, std::time::Duration};

#[test]
fn create_timestamp() {
    let unix_timestamp = Utc::now().timestamp_millis();

    // Wait for a short duration to ensure the timestamp is different.
    let d = 5i64;
    std::thread::sleep(Duration::from_millis(d as u64));

    let timestamp = HlcTimestamp::new(Utc::now().timestamp_millis()).unwrap();

    // Ensure the timestamp is within the expected range.
    assert!(timestamp.timestamp() - unix_timestamp >= d);
    assert!(timestamp.timestamp() - unix_timestamp <= d * 2);
    assert_eq!(timestamp.count(), 0);
}

#[test]
fn from_parts() {
    let timestamp = HlcTimestamp::from_parts(EPOCH + 12345, 67890).unwrap();
    assert_eq!(timestamp.timestamp(), EPOCH + 12345);
    assert_eq!(timestamp.count(), 67890);
}

#[test]
fn from_and_to_u64() {
    let timestamp = HlcTimestamp::from_parts(EPOCH + 12345, 67890).unwrap();
    let u64_timestamp = timestamp.as_u64();
    let back_to_timestamp: HlcTimestamp = u64_timestamp.try_into().unwrap();

    assert_eq!(timestamp, back_to_timestamp);
}
