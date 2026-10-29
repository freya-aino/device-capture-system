# Step 05: Remove panics on the network receive path

- **Status:** ✅ Done
- **Stage:** 1 — Transport and hub foundation
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 5

## Scope

Peer-controlled input on the network receive path must not cause a panic. Malformed frames are dropped with a log message, and dropped-frame counts come from sequence gaps.

## Evidence

- `networking/src/receiver.rs` drops frames with fewer than 3 parts, a malformed sequence number, an undecodable header, or no `tx_timestamp`.
- `networking/src/receiver.rs` derives `dropped_frames` from sequence gaps.
- `networking/tests/receiver-malformed-test.rs` sends short, bad-sequence and bad-header messages and checks that a following valid frame is still delivered.
- The `assert!` calls in `receive()` check local state (connection status and socket), not peer input.

## Step checklist

- [x] Frames with fewer than 3 parts are dropped (`networking/src/receiver.rs`)
- [x] Malformed sequence number, undecodable header and missing `tx_timestamp` are dropped
- [x] Dropped-frame count is derived from sequence gaps
- [x] Dedicated test that sends malformed multipart messages and asserts no panic (`networking/tests/receiver-malformed-test.rs`)

## Stage 1 definition of done

- [x] `Proxy` forwards frames between XSUB and XPUB and stops cleanly
- [x] Peer-controlled input on the receive path is dropped, not panicked on
- [x] `ConnectionStats` uses `Instant` and records per-frame latency
- [x] `cargo test --workspace` passes (run 2026-10-30 at `3c3d183`, 13 test binaries, all passing)
