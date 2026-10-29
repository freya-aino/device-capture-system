# Step 08: Camera buffer count reduced to 4 and format validation

- **Status:** ✅ Done
- **Stage:** 2 — Producers (devices and device-node)
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 8

## Scope

Reduce the camera buffer count from 128 to 4, and validate the negotiated v4l2 format before capture starts.

## Evidence

- `devices/src/camera.rs` uses 4 buffers (confirmed in the last review).
- `devices/src/camera.rs` validates the negotiated format (confirmed in the last review).

## Step checklist

- [x] Camera uses 4 buffers (`devices/src/camera.rs`)
- [x] Negotiated v4l2 format is validated (`devices/src/camera.rs`)
- [ ] Capture verified on real hardware — 2026-10-30: the ignored streaming test delivered 42 frames in 3 s (14 fps) at 1920×1080 MJPG; `device-node run camera` fails because its fixed default (1920×1080 YUYV) is not advertised ([results](../../hardware/2026-10-30-framework-12-webcam.md), fix tracked as [step 19](step-19-camera-config-negotiation.md))

## Stage 2 definition of done

- [x] Camera and microphone enumeration filters are in place
- [x] Camera and microphone report `Running`/`Stopped`, and `device-node` serves `GET /status`
- [x] `DeviceNode` is generic over `T: Device`
- [x] Remote `/start`, `/stop`, `/restart` return `409` on invalid transitions
- [ ] Capture loops exercised on real hardware (moved to the hardware validation stage)
- [x] `cargo test --workspace` passes (run 2026-10-30 at `3c3d183`, 13 test binaries, all passing)
