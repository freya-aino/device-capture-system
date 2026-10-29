# Step 02: Minimal `control-hub` `main()` loads config and runs `Proxy`

- **Status:** ✅ Done
- **Stage:** 1 — Transport and hub foundation
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 2

## Scope

The `control-hub` binary loads its bind and connect settings from `HUB_*` environment variables and runs `networking::Proxy` between producers and consumers.

## Evidence

- `control-hub/src/main.rs` loads config from the environment and runs `Proxy` (confirmed in the last review).

## Step checklist

- [x] `control-hub/src/main.rs` loads config from `HUB_*` environment variables
- [x] `control-hub/src/main.rs` runs `networking::Proxy`

## Stage 1 definition of done

- [x] `Proxy` forwards frames between XSUB and XPUB and stops cleanly
- [x] Peer-controlled input on the receive path is dropped, not panicked on
- [x] `ConnectionStats` uses `Instant` and records per-frame latency
- [ ] `cargo test --workspace` passes (not run in the last review)
