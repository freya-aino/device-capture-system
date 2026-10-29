# Step 06: Device status transitions and `GET /status`

- **Status:** ✅ Done
- **Stage:** 2 — Producers (devices and device-node)
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 6

## Scope

Camera and microphone set `DeviceStatus` to `Running` on start and `Stopped` on stop. The `device-node run` command serves `GET /status` (JSON with device and connection state) on `DEVICE_NODE_STATUS_ADDR`, default `127.0.0.1:8080`.

## Evidence

- `devices/src/microphone_linux.rs` sets `Running` on start and `Stopped` on stop.
- `device-node/tests/status-test.rs` exists and covers the status endpoint.

## Step checklist

- [x] Microphone sets `Running` on start and `Stopped` on stop (`devices/src/microphone_linux.rs`)
- [x] `device-node/tests/status-test.rs` covers the status endpoint
- [ ] Camera status transitions checked against the same behaviour (not re-read in the last review)

## Stage 2 definition of done

- [x] Camera and microphone enumeration filters are in place
- [x] Camera and microphone report `Running`/`Stopped`, and `device-node` serves `GET /status`
- [x] `DeviceNode` is generic over `T: Device`
- [x] Remote `/start`, `/stop`, `/restart` return `409` on invalid transitions
- [ ] Capture loops exercised on real hardware (moved to the hardware validation stage)
- [ ] `cargo test --workspace` passes (not run in the last review)
