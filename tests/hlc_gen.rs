use {
    hlc_gen::{CurrentTimestamp, HlcGenerator, HlcTimestamp, ManualTimestamp},
    parking_lot::Mutex,
    std::{sync::Arc, time::Duration},
};

#[test]
fn timstamp_ordering() {
    let g = HlcGenerator::default();

    let t1 = g.next_timestamp().unwrap();
    let t2 = g.next_timestamp().unwrap();
    assert!(t1 < t2);

    // Simulate a delay
    std::thread::sleep(Duration::from_nanos(1));
    let t3 = g.next_timestamp().unwrap();
    assert_eq!(t3.count(), 0);
    assert!(t2 < t3);
}

#[test]
fn manual_current_time() {
    let g = HlcGenerator::<ManualTimestamp>::new();

    g.ts_provider().set_current_timestamp(42);
    let t1 = g.next_timestamp().unwrap();
    assert_eq!(t1.timestamp(), 42);
    assert_eq!(t1.count(), 0);

    g.ts_provider().set_current_timestamp(43);
    let t1 = g.next_timestamp().unwrap();
    assert_eq!(t1.timestamp(), 43);
    assert_eq!(t1.count(), 0);
}

#[test]
fn max_drift() {
    let max_drift = 1000;

    let g = HlcGenerator::<ManualTimestamp>::new();

    g.ts_provider().set_current_timestamp(12345);
    g.set_max_drift(max_drift as usize);

    let t1 = g.next_timestamp().unwrap();
    assert_eq!(t1.timestamp(), 12345);

    let t2 = HlcTimestamp::from_parts(12345 + max_drift, 2);
    let t3 = HlcTimestamp::from_parts(12345 + max_drift + 1, 5);

    // Check error produced only when the drift is exceeded.
    assert_eq!(
        g.update(&t2),
        Ok(HlcTimestamp::from_parts(12345 + max_drift, 3))
    );
    assert_eq!(
        g.update(&t3),
        Err(hlc_gen::error::HlcError::DriftTooLarge(
            max_drift as usize + 1,
            max_drift as usize
        ))
    );

    // Disable the drift check.
    g.set_max_drift(0);
    assert_eq!(
        g.update(&t3),
        Ok(HlcTimestamp::from_parts(12345 + max_drift + 1, 6))
    );
}

#[test]
fn multi_step() {
    let max_drift = 1000;
    enum TestCase {
        // Set local clock, check expected next timestamp (to be used in local event or for
        // sending to remote node)
        Send(i64, HlcTimestamp),
        // Set local clock, provide remote timestamp, check expected next timestamp on update.
        Receive(i64, HlcTimestamp, HlcTimestamp),
    }

    use TestCase::*;

    let tests = vec![
        // Warm-up
        Send(5, HlcTimestamp::from_parts(5, 0)),
        Send(6, HlcTimestamp::from_parts(6, 0)),
        Receive(
            10,
            HlcTimestamp::from_parts(10, 5),
            HlcTimestamp::from_parts(10, 6),
        ),
        // Local clock jumps back.
        Send(5, HlcTimestamp::from_parts(10, 7)),
        // Same wall-clock time, local logical count is higher.
        Receive(
            6,
            HlcTimestamp::from_parts(10, 4),
            HlcTimestamp::from_parts(10, 8),
        ),
        // Same wall-clock time, remote logical count is higher.
        Receive(
            7,
            HlcTimestamp::from_parts(10, 12),
            HlcTimestamp::from_parts(10, 13),
        ),
        // Incoming message which has drifted too far, discard.
        Receive(
            8,
            HlcTimestamp::from_parts(10 + max_drift + 1, 14),
            HlcTimestamp::from_parts(10, 13),
        ),
        // Physical clock higher than local timestamp, update local timestamp.
        Receive(
            11,
            HlcTimestamp::from_parts(10, 45),
            HlcTimestamp::from_parts(11, 0),
        ),
        Send(11, HlcTimestamp::from_parts(11, 1)),
    ];

    let g = HlcGenerator::<ManualTimestamp>::with_max_drift(max_drift as usize);
    for test in tests {
        match test {
            Send(ts, expected) => {
                g.ts_provider().set_current_timestamp(ts);
                let t1 = g.next_timestamp().unwrap();
                assert_eq!(t1, expected);
            }
            Receive(ts, incoming_timestamp, expected) => {
                g.ts_provider().set_current_timestamp(ts);
                let t1 = g.timestamp();
                let res = g.update(&incoming_timestamp);
                if let Ok(t2) = res {
                    assert_ne!(t1, t2, "timestamp not updated, no error reported");
                    assert_eq!(t2, expected);
                } else {
                    continue;
                }
            }
        }
    }
}

#[test]
fn multi_threaded_logical_clock_updated() {
    // Generate timestamps in a multi-threaded environment, at some point producing
    // more than a single timestamp in the same nanosecond. This will test the
    // logical clock update.

    let g = Arc::new(HlcGenerator::default());
    let mut handles = vec![];
    let timestamps = Arc::new(Mutex::new(vec![]));
    for _ in 0..100 {
        let g = g.clone();
        let timestamps = timestamps.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..100 {
                let t = g.next_timestamp().unwrap();
                timestamps.lock().push(t);
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }

    let mut timestamps = Arc::try_unwrap(timestamps).unwrap().into_inner();
    timestamps.sort();

    let non_zero_count = timestamps.iter().filter(|t| t.count() != 0).count();
    assert!(non_zero_count > 0);

    // Ensure that all timestamps are unique and in order.
    let mut prev = None;
    for t in timestamps.into_iter() {
        if let Some(p) = prev {
            assert!(p < t);
        }
        prev = Some(t);
    }
}
