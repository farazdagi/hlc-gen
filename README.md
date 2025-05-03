# hlc-gen

[![crates.io](https://img.shields.io/crates/d/hlc-gen.svg)](https://crates.io/crates/hlc-gen)
[![docs.rs](https://docs.rs/hlc-gen/badge.svg)](https://docs.rs/hlc-gen)

Lock-free Hybrid Logical Clock (HLC) timestamp generator.

Implements the Hybrid Logical Clock (HLC) machinery outlined in
[Logical Physical Clocks and Consistent Snapshots in Globally Distributed Databases](http://www.cse.buffalo.edu/tech-reports/2014-04.pdf)
paper.

## Features

- [x] Lock-free implementation of the HLC algorithm.
- [x] High throughput, easy to use with minimal API (timestamp generation for local and send
  events + generator adjusting on receive events).

## Motivation

The idea is to have a generator producing timestamp-based IDs that are:

- unique and monotonic
- based on timestamp, thus allowing the correct ordering of events ("happened before" relationship
  preserved)
- have a small size (64 bits)
- operate on a millisecond granularity
- are generated in a lock-free manner

## Usage

``` rust
use hlc_gen::{HlcGenerator, HlcTimestamp};

// HLC timestamp generator (with 1000ms of allowable drift).
let g = HlcGenerator::new(1000);

// Generate a timestamp to either mark some local event
// or to send it to another node.
let ts1: HlcTimestamp = g.next_timestamp()
                         .expect("Failed to generate timestamp");

// When message comes from another node, we can update
// the generator to preserve the causality relationship.
let ts2 = HlcTimestamp::from_parts(1_704_067_200_042, 12345)
                       .expect("Failed to create timestamp");
g.update(&ts2)
 .expect("Incoming message has timestamp that is drifted too far");

// Newly generated timestamp will "happen after" both
// the previous local and incoming remote timestamps.
let ts3: HlcTimestamp = g.next_timestamp().unwrap();
assert!(ts3 > ts1);
assert!(ts3 > ts2);

// To obtain the wall-clock time in milliseconds (Unix timestamp),
// and the logical clock count, use:
let (ts, cnt) = ts3.parts();
let (ts, cnt) = (ts3.timestamp(), ts3.count());
```

## Implementation Details

The generated [`HlcTimestamp`](https://docs.rs/hlc-gen/latest/hlc_gen/struct.HlcTimestamp.html) is a
thin wrapper over `u64` and conceptually is split into two parts: wall-clock time and logical clock.

``` verbatim, ignore
 0                                   42                        64
 +------------------------------------+-------------------------+
 | Wall-clock time (in ms)            | Logical clock (counter) |
 +------------------------------------+-------------------------+
```

The logical clock is used to resolve the "happened before" relationship between events that occur at
the same millisecond, i.e. when granularity of the wall-clock time is not small enough to
distinguish between events.

### Lock-free Implementation

Internally, `AtomicU64` is used to store and update the state of the timestamp, where the first 42
bits are used for the wall-clock and the last 22 bits are used for the logical clock. The end-user
is not required to worry about the details of the implementation, as the API exposes only snapshots
of the state of the generator (via `HlcTimestamp`).

### Granularity and Number of Timestamps

The wall-clock time is stored as milliseconds from custom epoch (starts at 2024-01-01), and is
monotonically increasing, thus the 42 bits are enough to cover around 139 years of time. The logical
clock uses the remaining 22 bits, and it is enough to cover around 4M of items per millisecond.

### Arithmetic Operations

`HlcTimestamp` implements the `Add`, `Sub`, `AddAssign`, `SubAssign` traits, so you can update the
internal timestamp without creating a new instance.

``` rust
use hlc_gen::{HlcTimestamp};

let start = chrono::Utc::now().timestamp_millis();
let t1 = HlcTimestamp::from_parts(start, 123).unwrap();

// Add 1000ms to the timestamp, creating a new instance
let t2 = t1 + 1000;
assert_eq!(t2.timestamp(), start + 1000);
assert_eq!(t2.count(), 123);

// Subtract 1000ms from the timestamp, creating a new instance
let mut t3 = t2 - 1000;
assert_eq!(t3, t1);
assert_eq!(t3.timestamp(), start);
assert_eq!(t3.count(), 123);

// Add 1000ms to the timestamp
t3 += 1000;
assert_eq!(t3.timestamp(), start + 1000);
assert_eq!(t3.count(), 123);

// Subtract 1000ms from the timestamp
t3 -= 1000;
assert_eq!(t3, t1);
assert_eq!(t3.timestamp(), start);

// Find the difference between two timestamps (in ms)
assert_eq!(t2 - t1, 1000);
assert_eq!(t1 - t2, -1000);
assert_eq!(t3 - t3, 0);
```

## Sample Use Case

Since HLC timestamps are based on the wall-clock time, they are quite useful in algorithms that
require ordering of events, or that need to determine how far apart two events are in time.

For example, HLC timestamps can be used to implement page replacement algorithms, where page
accesses are stamped with them. This allows an algorithm to determine which pages are good
candidates for eviction, based on access patterns: e.g. remove the least recently accessed page, but
making sure that it has been added more than 5 minute ago (based on
[Jim Gray's 5 minute rule](https://dl.acm.org/doi/10.1145/38714.38755) idea ).

## License

MIT
