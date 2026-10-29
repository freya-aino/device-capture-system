# Step 04: Microphone enumeration fix and camera capability filter

- **Status:** ✅ Done
- **Stage:** 2 — Producers (devices and device-node)
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 4

## Scope

Fix microphone enumeration so playback-only hints are skipped and virtual devices are matched by base name. Add a camera capability filter for `V4L2_CAP_VIDEO_CAPTURE`. The hardware itself is not exercised by this step.

## Evidence

- `devices/src/microphone_linux.rs` skips `Direction::Playback` hints and matches virtual devices (`default`, `null`, `pipewire`, `sysdefault`) by base name.
- `devices/src/microphone_linux.rs` multiplies the buffer size by the channel count.
- `devices/src/camera.rs` applies the `VIDEO_CAPTURE` capability filter.

## Step checklist

- [x] Microphone: playback-only hints are skipped (`devices/src/microphone_linux.rs`)
- [x] Microphone: virtual devices are matched by base name
- [x] Microphone: buffer size accounts for channel count
- [x] Camera: `VIDEO_CAPTURE` capability filter (`devices/src/camera.rs`)
- [ ] Enumeration verified on real hardware (carried into the hardware validation stage)

## Stage 2 definition of done

- [x] Camera and microphone enumeration filters are in place
- [x] Camera and microphone report `Running`/`Stopped`, and `device-node` serves `GET /status`
- [x] `DeviceNode` is generic over `T: Device`
- [x] Remote `/start`, `/stop`, `/restart` return `409` on invalid transitions
- [ ] Capture loops exercised on real hardware (moved to the hardware validation stage)
- [ ] `cargo test --workspace` passes (not run in the last review)
