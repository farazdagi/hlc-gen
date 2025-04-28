# hlc-gen

Lock-free Hybrid Logical Clock (HLC) generator

Implements the Hybrid Logical Clock (HLC) outlined in
[Logical Physical Clocks and Consistent Snapshots in Globally Distributed Databases](http://www.cse.buffalo.edu/tech-reports/2014-04.pdf)
paper.

## Features

- [ ] Lock-free implementation of the HLC algorithm.
- [ ] High throughput, easy to use in a single node context to generate unique timestamp based
  sequence of IDs.
- [ ] Support for distributed applications with a single node as a clock source.

## Motivation

Very lightweight and fast HLC generator suitable for both distributed and single-node applications.

In context of single-node applications, it can be used as a high-performance alternative to unique
IDs (be it ULIDs, UUIDs, TSIDs) generators. The idea is to have physical clock pacing with "happens
before" causality established, timestamp sorting and uniqueness guarantees, operate on a
sub-millisecond level (nanoseconds), and be able to generate them in a lock-free manner.

For distributed applications, updating mechanism of the paper is also

## Usage

``` rust
use hlc_gen::{HlcGenerator, HlcTimestamp};

// Create a new HLC generator
let g = HlcGenerator::new();

// Generate a new HLC timestamp
let ts: HlcTimestamp = g.next_timestamp().expect("Failed to generate timestamp");

// When message comes from another node, we can update
// the local clock to preserve the causal relationship.
let another_ts: HlcTimestamp = HlcTimestamp::new(1234567890, 1234567890);
g.update(ts).expect("Incoming message has timestamp that is drifted too far");

// Newly generated timestamp will "happen after" both
// the previous local and incoming remote timestamps.
let ts: Option<HlcTimestamp> = g.next_timestamp();

```
