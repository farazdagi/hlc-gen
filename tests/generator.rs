mod common;

use {
    common::EPOCH,
    hlc_gen::{HlcGenerator, HlcTimestamp},
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
    std::thread::sleep(Duration::from_millis(1));
    let t3 = g.next_timestamp().unwrap();
    assert_eq!(t3.count(), 0);
    assert!(t2 < t3);
}

#[test]
fn manual_current_time() {
    let g = HlcGenerator::manual(0);

    g.set_current_timestamp(EPOCH + 42);
    let t1 = g.next_timestamp().unwrap();
    assert_eq!(t1.timestamp(), EPOCH + 42);
    assert_eq!(t1.count(), 0);

    g.set_current_timestamp(EPOCH + 43);
    let t1 = g.next_timestamp().unwrap();
    assert_eq!(t1.timestamp(), EPOCH + 43);
    assert_eq!(t1.count(), 0);
}

#[test]
fn max_drift() {
    let max_drift = 1000;

    let g = HlcGenerator::manual(max_drift as usize);
    g.set_current_timestamp(EPOCH + 12345);

    let t1 = g.next_timestamp().unwrap();
    assert_eq!(t1.timestamp(), EPOCH + 12345);
    assert_eq!(t1.count(), 0);

    let t2 = HlcTimestamp::from_parts(EPOCH + 12345 + max_drift, 2).unwrap();
    let t3 = HlcTimestamp::from_parts(EPOCH + 12345 + max_drift + 1, 5).unwrap();

    // Check error produced only when the drift is exceeded.
    assert_eq!(
        g.update(&t2),
        Ok(HlcTimestamp::from_parts(EPOCH + 12345 + max_drift, 3).unwrap())
    );
    assert_eq!(
        g.update(&t3),
        Err(hlc_gen::error::HlcError::DriftTooLarge(
            max_drift as usize + 1,
            max_drift as usize
        ))
    );

    // With max_drift set to 0, the drift check is ignored.
    let g = HlcGenerator::manual(0);
    g.set_current_timestamp(EPOCH + 12345);
    assert_eq!(
        g.update(&t3),
        Ok(HlcTimestamp::from_parts(EPOCH + 12345 + max_drift + 1, 6).unwrap())
    );
}

#[test]
fn multi_step() {
    let max_drift = 1000;
    #[derive(Debug)]
    enum TestCase {
        // Set local clock, check expected next timestamp (to be used in local event or for
        // sending to remote node)
        Send(i64, (i64, u64)),
        // Set local clock, provide remote timestamp, check expected next timestamp on update.
        Receive(i64, (i64, u64), (i64, u64)),
    }

    use TestCase::*;

    let tests = vec![
        // Warm-up
        Send(5, (5, 0)),
        Send(6, (6, 0)),
        Receive(10, (10, 5), (10, 6)),
        // Local clock jumps back.
        Send(5, (10, 7)),
        // Same wall-clock time, local logical count is higher.
        Receive(6, (10, 4), (10, 8)),
        // Same wall-clock time, remote logical count is higher.
        Receive(7, (10, 12), (10, 13)),
        // Incoming message which has drifted too far, discard.
        Receive(8, (10 + max_drift + 1, 14), (10, 13)),
        // Physical clock higher than local timestamp, update local timestamp.
        Receive(11, (10, 45), (11, 0)),
        Send(11, (11, 1)),
    ];

    let g = HlcGenerator::manual(max_drift as usize);
    for test in tests {
        match test {
            Send(ts, expected) => {
                let expected = HlcTimestamp::from_parts(EPOCH + expected.0, expected.1).unwrap();
                g.set_current_timestamp(EPOCH + ts);
                let t1 = g.next_timestamp().unwrap();
                assert_eq!(t1, expected);
            }
            Receive(ts, incoming_timestamp, expected) => {
                let expected = HlcTimestamp::from_parts(EPOCH + expected.0, expected.1).unwrap();
                let incoming_timestamp =
                    HlcTimestamp::from_parts(EPOCH + incoming_timestamp.0, incoming_timestamp.1)
                        .unwrap();
                g.set_current_timestamp(EPOCH + ts);
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
    // more than a single timestamp in the same millisecond. This will test the
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
                std::thread::sleep(Duration::from_nanos(1000));
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }

    let timestamps = Arc::try_unwrap(timestamps).unwrap().into_inner();

    // Ensure that at least some logical clocks got updated.
    let non_zero_count = timestamps
        .iter()
        .map(|t| t.count())
        .filter(|lc| *lc != 0)
        .count();
    assert!(non_zero_count > 0);

    // Ensure that all timestamps are in order i.e. no two timestamps are equal.
    let mut timestamps = timestamps
        .into_iter()
        // .map(|t| t.as_u64())
        .collect::<Vec<_>>();
    timestamps.sort();

    // Ensure that all timestamps are unique and in order.
    let mut prev = None;
    for t in timestamps.into_iter() {
        if let Some(p) = prev {
            assert!(p < t);
        }
        prev = Some(t);
    }
}
