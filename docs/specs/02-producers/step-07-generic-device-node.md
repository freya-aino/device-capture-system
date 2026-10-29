# Step 07: `DeviceNode` generic over `T: Device`

- **Status:** ✅ Done
- **Stage:** 2 — Producers (devices and device-node)
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 7

## Scope

`DeviceNode` is generic over any `T: Device`, so producer hosts can publish cameras as well as microphones.

## Evidence

- `device-node/tests/status-test.rs` uses a `Device` trait mock, which shows the node is generic over the trait.

## Step checklist

- [x] `DeviceNode` is generic over `T: Device`
- [x] `Device` trait mock is used in `device-node/tests/status-test.rs`

## Stage 2 definition of done

- [x] Camera and microphone enumeration filters are in place
- [x] Camera and microphone report `Running`/`Stopped`, and `device-node` serves `GET /status`
- [x] `DeviceNode` is generic over `T: Device`
- [x] Remote `/start`, `/stop`, `/restart` return `409` on invalid transitions
- [ ] Capture loops exercised on real hardware (moved to the hardware validation stage)
- [x] `cargo test --workspace` passes (run 2026-10-30 at `3c3d183`, 13 test binaries, all passing)
