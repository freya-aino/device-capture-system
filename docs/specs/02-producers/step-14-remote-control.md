# Step 14: Remote `POST /start`, `/stop`, `/restart` with 409 on invalid transitions

- **Status:** ✅ Done
- **Stage:** 2 — Producers (devices and device-node)
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 14

## Scope

`device-node` accepts `POST /start`, `POST /stop` and `POST /restart` on the status server. Transitions are validated against `DeviceStatus` before the device is called, and invalid transitions return `409`. Device failures return `500`.

## Evidence

- `device-node/tests/status-test.rs` covers the `409` cases: start while running, stop while stopped, restart while stopped, and start on a never-started node.
- `SPECS.md` §7.1 documents the response formats.

## Step checklist

- [x] Transitions are validated before the device is called
- [x] `409` tests in `device-node/tests/status-test.rs`, including start on a never-started node
- [ ] `500` device-failure path (`device control failed`) covered by a test (not confirmed in the last review)

## Stage 2 definition of done

- [x] Camera and microphone enumeration filters are in place
- [x] Camera and microphone report `Running`/`Stopped`, and `device-node` serves `GET /status`
- [x] `DeviceNode` is generic over `T: Device`
- [x] Remote `/start`, `/stop`, `/restart` return `409` on invalid transitions
- [ ] Capture loops exercised on real hardware (moved to the hardware validation stage)
- [ ] `cargo test --workspace` passes (not run in the last review)
