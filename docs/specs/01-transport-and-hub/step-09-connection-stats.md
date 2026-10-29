# Step 09: `ConnectionStats` uses `Instant` and records per-frame latency

- **Status:** ✅ Done
- **Stage:** 1 — Transport and hub foundation
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 9

## Scope

Rewrite `ConnectionStats` to use `Instant` for interval math, and record per-frame latency from `tx_timestamp` and `rx_timestamp`.

## Evidence

- `networking/src/receiver.rs` records latency with `connection.stats.record_latency(...)` on every accepted frame.
- The `ConnectionStats` definition in the `shared` crate was not inspected in the last review.

## Step checklist

- [x] Receive path records per-frame latency into `connection.stats` (`networking/src/receiver.rs`)
- [ ] `shared` `ConnectionStats` uses `Instant` for interval math (not inspected in the last review)

## Stage 1 definition of done

- [x] `Proxy` forwards frames between XSUB and XPUB and stops cleanly
- [x] Peer-controlled input on the receive path is dropped, not panicked on
- [x] `ConnectionStats` uses `Instant` and records per-frame latency
- [ ] `cargo test --workspace` passes (not run in the last review)
