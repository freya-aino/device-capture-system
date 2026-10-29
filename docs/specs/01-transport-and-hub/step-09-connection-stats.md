# Step 09: `ConnectionStats` uses `Instant` and records per-frame latency

- **Status:** ✅ Done
- **Stage:** 1 — Transport and hub foundation
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 9

## Scope

Rewrite `ConnectionStats` to use `Instant` for interval math, and record per-frame latency from `tx_timestamp` and `rx_timestamp`.

## Evidence

- `networking/src/receiver.rs` records latency with `connection.stats.record_latency(...)` on every accepted frame.
- `shared/src/networking.rs` keeps `first_frame_time: Option<Instant>` and exposes `record_latency(Duration)`, with a unit test for the stored seconds.

## Step checklist

- [x] Receive path records per-frame latency into `connection.stats` (`networking/src/receiver.rs`)
- [x] `shared` `ConnectionStats` uses `Instant` for interval math (`shared/src/networking.rs`, `first_frame_time: Option<Instant>`)

## Stage 1 definition of done

- [x] `Proxy` forwards frames between XSUB and XPUB and stops cleanly
- [x] Peer-controlled input on the receive path is dropped, not panicked on
- [x] `ConnectionStats` uses `Instant` and records per-frame latency
- [x] `cargo test --workspace` passes (run 2026-10-30 at `3c3d183`, 13 test binaries, all passing)
