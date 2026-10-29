# Step 04: Microphone enumeration fix and camera capability filter

- **Status:** ⚠️ Partial (filters exist; they drop every hardware hint on a PipeWire host, see step 20)
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
- [ ] Enumeration verified on real hardware — 2026-10-30: camera enumerates (`/dev/video0`), microphone list is empty on a host with one capture PCM because the `DEV` filter drops `front:CARD=PCH,DEV=0` ([results](../../hardware/2026-10-30-framework-12-webcam.md), fix tracked as [step 20](step-20-microphone-enumeration-pipewire.md))

## Stage 2 definition of done

- [x] Camera and microphone enumeration filters are in place
- [x] Camera and microphone report `Running`/`Stopped`, and `device-node` serves `GET /status`
- [x] `DeviceNode` is generic over `T: Device`
- [x] Remote `/start`, `/stop`, `/restart` return `409` on invalid transitions
- [ ] Capture loops exercised on real hardware (moved to the hardware validation stage)
- [x] `cargo test --workspace` passes (run 2026-10-30 at `3c3d183`, 13 test binaries, all passing)
