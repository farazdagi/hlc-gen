# hlc-gen

[![crates.io](https://img.shields.io/crates/d/hlc-gen.svg)](https://crates.io/crates/hlc-gen)
[![docs.rs](https://docs.rs/hlc-gen/badge.svg)](https://docs.rs/hlc-gen)

Lock-free Hybrid Logical Clock (HLC) timestamp generator.

Implements the Hybrid Logical Clock (HLC) machinery outlined in
[Logical Physical Clocks and Consistent Snapshots in Globally Distributed Databases](http://www.cse.buffalo.edu/tech-reports/2014-04.pdf)
paper.

## Features

- [ ] Lock-free implementation of the HLC algorithm.
- [x] High throughput, easy to use with minimal API (timestamp generation for local and send
  events + generator adjusting on receive events).

## Motivation

The idea is to have a generator producing timestamp-based IDs that are:

- unique and monotonic
- based on timestamp, thus allowing the correct ordering of events ("happened before" relationship
  preserved)
- operate on a sub-millisecond level (produced timestamps are in nanoseconds)
- are generated in a lock-free manner

## Implementation Details

The generated HLC timestamp has two components (see
[`HlcTimestamp`](https://docs.rs/hlc-gen/latest/hlc_gen/struct.HlcTimestamp.html)):

``` rust
#[derive(Hash, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct HlcTimestamp {
    /// Wall-clock time.
    ///
    /// Timestamp in nanoseconds since the Unix epoch.
    pt: i64,

    /// The logical clock value.
    ///
    /// Captures causality for events that occur at the same wall-clock time.
    lc: u64,
}
```

Basically, you have two counters, one for the wall-clock time and another for the logical clock. The
logical clock is used to resolve the "happened before" relationship between events that occur at the
same, that is when granularity of the wall-clock time is not small enough to distinguish between
events.

Note that `HlcId` does not contain any information about the node that generated it. It is assumed
that whenever uniqueness across all nodes is required, the node ID should be sent along with the
`HlcId`.

The aim of this crate is to provide the most simple and efficient implementation of the HLC
algorithm -- as it is outlined in the paper. If such an approach is not enough, it is worth
considering other, more complex, algorithms like `ULID`, `UUID` (ver. 7), or `TSID`.

## Sample Use Case

Since HLC timestamps are based on the wall-clock time, they are quite useful in algorithms that
require ordering of events, or that need to determine how far apart two events are in time.

For example, HLC timestamps can be used to implement page replacement algorithms, where page
accesses are marked with HLC timestamps. This allows the algorithm to determine which pages are good
candidates for eviction based on their access patterns: remove the least recently accessed page, but
making sure that it has been added more than 5 minute ago (based on Jim Gray's 5 minute rule idea,
see
[The 5 minute rule for trading memory for disc accesses and the 10 byte rule for trading memory for CPU time](https://dl.acm.org/doi/10.1145/38714.38755)
paper).

## Usage

``` rust
use hlc_gen::{HlcGenerator, HlcTimestamp};

// Create a new HLC generator.
let g = HlcGenerator::new();

// Generate a new HLC timestamp for local or send event.
let ts: HlcTimestamp = g.next_timestamp()
                        .expect("Failed to generate timestamp");

// When message comes from another node, we can update
// the generator to preserve the causality relationship.
let another_ts = HlcTimestamp::from_parts(1234567890, 1234567890);
g.update(ts)
 .expect("Incoming message has timestamp that is drifted too far");

// Newly generated timestamp will "happen after" both
// the previous local and incoming remote timestamps.
let ts: Option<HlcTimestamp> = g.next_timestamp();

```
