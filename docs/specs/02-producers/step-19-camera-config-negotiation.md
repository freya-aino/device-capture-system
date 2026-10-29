# Step 19: Camera config negotiation and FourCC on the wire

- **Status:** ❌ Open
- **Stage:** 2 — Producers (devices and device-node)
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 19

## Why this step exists

The 2026-10-30 hardware run ([results](../../hardware/2026-10-30-framework-12-webcam.md)) showed that the
implementation the specs proposed for the camera producer does not start on a real UVC webcam:

- `device-node run camera` passes the fixed `DEFAULT_CAMERA_CONFIG` (1920×1080 YUYV at 30 fps) to the device and
  `V4lCameraDevice::start` now refuses any other negotiated format, so the node exits with
  `Camera negotiated 640x480 YUYV instead of requested 1920x1080 YUYV`.
- `shared::select_camera_config` and `select_camera_config_by_policy` (items added at `bd10290` and `d65f446`) pick
  from the advertised list, but nothing in `device-node` calls them.
- `select_camera_config` ignores FourCC, so on this camera it chooses 1920×1080 MJPG. The wire header
  (`FramePacketInformation`) has no pixel-format field, and `SPECS.md` §7 says only "raw v4l2 buffer in the negotiated
  FourCC". A consumer receiving MJPG and YUYV frames cannot tell them apart.

## Scope

1. `device-node` selects the camera config from `get_configs()` instead of a fixed default. The requested
   resolution and frame rate (default 1920×1080 at 30 fps) and the selection policy come from CLI arguments or
   `DEVICE_NODE_CAMERA_*` environment variables; the chosen config is printed at start.
2. The chosen FourCC travels with every frame. Add a pixel-format field to `FramePacketInformation` (camera only;
   empty for microphones) and document it in `SPECS.md` §7. This is a wire-protocol change, so bump the protocol in
   the same change and update the Python `Frame` to expose it.
3. Decide, and write down in `SPECS.md` §9, whether camera-native compressed formats (MJPG) are allowed. The
   non-goal "no codecs in the hot path" refers to encoding added by this system; a camera that only offers MJPG at
   1080p forces the choice between accepting MJPG from the device or capping YUYV at 640×480.
4. Investigate the delivered 14 fps against a granted 1/30 interval and record the cause in the hardware note.

Acceptance: `device-node run camera 0 <hub>` starts on the Framework webcam and publishes frames whose header
carries the negotiated FourCC; a hardware-free test covers config selection with a mocked advertised list; the
Python `Frame` exposes the format.

## Step checklist

- [ ] `device-node` picks the camera config from the advertised list (policy and target from CLI/env)
- [ ] `FramePacketInformation` carries the pixel format; `SPECS.md` §7 updated
- [ ] Python `Frame` exposes the pixel format
- [ ] MJPG decision recorded in `SPECS.md` §9
- [ ] Delivered-rate shortfall diagnosed and recorded
- [ ] `device-node run camera` starts on the Framework webcam

## Stage 2 definition of done (extended)

- [x] Camera and microphone enumeration filters are in place
- [x] Camera and microphone report `Running`/`Stopped`, and `device-node` serves `GET /status`
- [x] `DeviceNode` is generic over `T: Device`
- [x] Remote `/start`, `/stop`, `/restart` return `409` on invalid transitions
- [ ] `device-node` starts a real camera without hand-editing the default config (this step)
- [ ] `device-node list` finds a real microphone (step 20)
- [ ] Capture loops exercised on real hardware (hardware validation stage)
- [x] `cargo test --workspace` passes (run 2026-10-30 at `3c3d183`, 13 test binaries, all passing)
