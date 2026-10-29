# Step 10: Time alignment across producers

- **Status:** ⚠️ Partial
- **Stage:** 5 — Sync
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 10

## Scope

`shared::Aligner` groups frames by capture time across producers. `control-hub` runs it in an `AlignedRelay` when `HUB_SYNC_PORT` and `HUB_SYNC_DEVICES` are set. Aligned groups are published on that port, and the raw forwarding stays unchanged.

Still open: validation with real devices, and a consumer that reads the aligned port by default. Calibration is tracked separately as item 17.

## Evidence

- `networking/src/aligned.rs` exists.
- `networking/tests/aligned-relay-test.rs` and `access-client/tests/aligned-client-test.rs` exist.
- The README marks sync as partial (`[~]`).

## Step checklist

- [x] `shared::Aligner` groups frames by capture time, reports skew and counts dropped stale samples
- [x] Hub `AlignedRelay` runs when `HUB_SYNC_PORT` and `HUB_SYNC_DEVICES` are set
- [x] Raw forwarding is unchanged while the relay is on
- [x] Relay and client tests exist (`networking/tests/aligned-relay-test.rs`, `access-client/tests/aligned-client-test.rs`)
- [ ] Validated with real devices
- [ ] Consumer reads the aligned port by default (today it is opt-in through `AlignedClient` or `GroupReceiver`)

## Stage 5 definition of done

- [x] `shared::Aligner` groups frames by capture time, reports skew and counts stale drops
- [x] Hub `AlignedRelay` publishes aligned groups on `HUB_SYNC_PORT` with raw forwarding unchanged
- [x] Hardware-free aligned relay and client tests exist
- [ ] Validated with real devices
- [ ] Consumer reads the aligned port by default
