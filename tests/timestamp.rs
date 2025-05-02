mod common;

use {
    chrono::Utc,
    common::EPOCH,
    hlc_gen::HlcTimestamp,
    std::{sync::Arc, time::Duration},
};

#[test]
fn create_timestamp() {
    let unix_timestamp = Utc::now().timestamp_millis();

    // Wait for a short duration to ensure the timestamp is different.
    let d = 5i64;
    std::thread::sleep(Duration::from_millis(d as u64));

    let timestamp = HlcTimestamp::new();

    // Ensure the timestamp is within the expected range.
    assert!(timestamp.parts().0 - unix_timestamp >= d);
    assert!(timestamp.parts().0 - unix_timestamp <= d * 2);
    assert_eq!(timestamp.parts().1, 0);
}

#[test]
fn from_parts() {
    let timestamp = HlcTimestamp::from_parts(EPOCH + 12345, 67890).unwrap();
    assert_eq!(timestamp.parts(), (EPOCH + 12345, 67890));
}

#[test]
fn from_and_to_u64() {
    let timestamp = HlcTimestamp::from_parts(EPOCH + 12345, 67890).unwrap();
    let u64_timestamp = timestamp.as_u64();
    let back_to_timestamp: HlcTimestamp = u64_timestamp.try_into().unwrap();

    assert_eq!(timestamp, back_to_timestamp);
}

#[test]
fn concurrent_updates() {
    let timestamp = Arc::new(HlcTimestamp::new());

    // Create multiple threads to update the timestamp concurrently.
    let mut handles = vec![];
    for t in 0..10 {
        let timestamp_clone = Arc::clone(&timestamp);
        handles.push(std::thread::spawn(move || {
            for i in 0..100 {
                let _ = timestamp_clone.update(move |_pt, _lc| Ok((EPOCH + t * 100 + i, 67890)));
            }
        }));
    }
    // Wait for all threads to finish.
    for handle in handles {
        handle.join().unwrap();
    }

    // Check that the timestamp is updated correctly.
    let final_timestamp = timestamp.parts().0;

    assert!(final_timestamp >= EPOCH);
    assert!(final_timestamp <= EPOCH + 1000);

    // One of the threads made the last update, so possible values are in range
    // [EPOCH, EPOCH + 1000]
    assert!((final_timestamp + 1) % 100 == 0);
}
