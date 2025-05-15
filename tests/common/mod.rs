// Pre-calculated Unix timestamp (in ms) for 2024-01-01 00:00:00 UTC.
// HLC timestamps are using custom epoch, so incoming timestamps cannot be
// smaller than this.
pub const EPOCH: i64 = 1_704_067_200_000;
