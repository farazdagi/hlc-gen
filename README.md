# hlc-gen

[![crates.io](https://img.shields.io/crates/d/hlc-gen.svg)](https://crates.io/crates/hlc-gen)
[![docs.rs](https://docs.rs/hlc-gen/badge.svg)](https://docs.rs/hlc-gen)

Lock-free Hybrid Logical Clock (HLC) generator.

Implements the Hybrid Logical Clock (HLC) outlined in
[Logical Physical Clocks and Consistent Snapshots in Globally Distributed Databases](http://www.cse.buffalo.edu/tech-reports/2014-04.pdf)
paper.

## Features

- [ ] Lock-free implementation of the HLC algorithm.
- [ ] High throughput, easy to use unique hybrid timestamp generator.

## Motivation

The idea is to have a sequence of IDs that:

- are unique and monotonic
- include a timestamp, with correct ordering of creation events (happens before)
- operate on a sub-millisecond level (nanoseconds)
- are generated in a lock-free manner

This crate is not intended to be a general-purpose globally unique ID generator. However, if can be
used as a high-performance alternative to such generators (be it ULIDs, UUIDs, TSIDs) in some
specific settings, see [Sample Use Case](#sample-use-case) below.

## Sample Use Case

Consider a page replacement algorithm that decides which page to swap out based on access pattern
(how often page is accessed, how much time passed from the last access). When access events happen
we need to attach a timestamp to the event. Ideally, events should be ordered not only on physical
time, but logically as well (if two events happened at the same physical time, we need to increase
the logical counter -- thus providing timestamp uniqueness).

HLC timestamps are perfect for this.

The [`HlcTimestamp`](https://docs.rs/hlc-gen/latest/hlc_gen/struct.HlcTimestamp.html) consists of
two components: wall-clock timestamp (in nanoseconds) and logical counter (for events happening at
the exact same instant). That's why it is called "hybrid" -- it combines physical and logical clocks
in a single structure.

The generator is capable of producing such timestamps in a lock-free manner, covering all the
outlined requirements. Additionally, it can adjust its local clocks (both wall and logical) when
used in distributed settings, and there are incoming messages marked with timestamps generated on other nodes.

## Usage

``` rust
use hlc_gen::{HlcGenerator, HlcTimestamp};

// Create a new HLC generator
let g = HlcGenerator::new();

// Generate a new HLC timestamp
let ts: HlcTimestamp = g.next_timestamp()
                        .expect("Failed to generate timestamp");

// When message comes from another node, we can update
// the local clock to preserve the causal relationship.
let another_ts = HlcTimestamp::new(1234567890, 1234567890);
g.update(ts)
 .expect("Incoming message has timestamp that is drifted too far");

// Newly generated timestamp will "happen after" both
// the previous local and incoming remote timestamps.
let ts: Option<HlcTimestamp> = g.next_timestamp();

```
