# Step 10: Time alignment across producers

- **Status:** ✅ Done (2026-10-30; validated on real devices, single host)
- **Stage:** 5 — Sync
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 10

## Scope

`shared::Aligner` groups frames by capture time across producers. `control-hub` runs it in an `AlignedRelay` when `HUB_SYNC_PORT` and `HUB_SYNC_DEVICES` are set. Aligned groups are published on that port, and the raw forwarding stays unchanged.

Frames are aligned on `capture_timestamp` (falling back to `tx_timestamp`), which the devices stamp with the real
capture time. The relay keeps a sample for `HUB_SYNC_MAX_WAIT_MS` (default 200) while waiting for partners and drops
it at once when every other device has moved past its window. The `access-client` CLI reads the aligned port with
`CLIENT_ALIGNED_PORT`. Calibration is tracked separately as item 17.

The original proposal said the consumer should read the aligned port *by default*. That was not done: the raw
stream is what single-device deployments, the Python bindings and the existing tests consume, and switching the
default would silently break them. Aligned reading is one environment variable away instead.

## Evidence

- `networking/src/aligned.rs` exists.
- `networking/tests/aligned-relay-test.rs` and `access-client/tests/aligned-client-test.rs` exist.
- The README marks sync as partial (`[~]`).

## Step checklist

- [x] `shared::Aligner` groups frames by capture time, reports skew and counts dropped stale samples
- [x] Hub `AlignedRelay` runs when `HUB_SYNC_PORT` and `HUB_SYNC_DEVICES` are set
- [x] Raw forwarding is unchanged while the relay is on
- [x] Relay and client tests exist (`networking/tests/aligned-relay-test.rs`, `access-client/tests/aligned-client-test.rs`)
- [x] Validated with real devices: 70 s, 2101 groups = one per camera frame at 30.0/s, skew mean 8 ms max 18 ms ([results](../../hardware/2026-10-30-framework-12-webcam.md), "Sync run")
- [x] Frames carry `capture_timestamp`; sync aligns on it (the first real-device run paired only 1 camera frame in 3 because microphone packets arrive four at a time with one send time)
- [x] `Aligner` separates the pairing tolerance from the wait window (`max_wait`, `HUB_SYNC_MAX_WAIT_MS`) and drops an anchor only once every other device has moved past it; replay test with the measured timing
- [x] Consumer reads the aligned port through `CLIENT_ALIGNED_PORT` (opt-in by decision, see Scope); `AlignedClient` and `GroupReceiver` remain for library users

## Stage 5 definition of done

- [x] `shared::Aligner` groups frames by capture time, reports skew and counts stale drops
- [x] Hub `AlignedRelay` publishes aligned groups on `HUB_SYNC_PORT` with raw forwarding unchanged
- [x] Hardware-free aligned relay and client tests exist
- [x] Validated with real devices
- [x] Consumer can read the aligned port (`CLIENT_ALIGNED_PORT`; opt-in by decision)
