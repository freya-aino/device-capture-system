# Step 12: Hardware validation of the capture loops

- **Status:** ✅ Done (2026-10-30, single-host three-process run; a cross-host run is still worth doing when two machines are available)
- **Stage:** 6 — Hardware validation
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 12

## Scope

Run the `V4lCameraDevice` and `AlsaMicrophoneDevice` capture loops against real devices, end to end through `device-node` → `control-hub` → `access-client`.

Acceptance: frames arrive at the configured rate (30 fps camera, 16 kHz microphone) for at least 60 s. The dropped frame count and latency are recorded in a dated results note. Tests stay hardware-free.

## Evidence

- [`docs/hardware/2026-10-30-framework-12-webcam.md`](../../hardware/2026-10-30-framework-12-webcam.md), "End-to-end
  run": 70 s through `device-node` (camera and microphone) → `control-hub` → `access-client` on one host. Camera
  2100 frames at 30.0 fps (1920×1080 MJPG), microphone 2188 packets at 16 013 Hz, 0 dropped frames, latency mean
  22–28 ms (camera) and 4–7 ms (microphone).
- The first device-side attempt found the blockers fixed in steps
  [19](../02-producers/step-19-camera-config-negotiation.md) and
  [20](../02-producers/step-20-microphone-enumeration-pipewire.md); the same note records them.
- The measurements come from the `access-client` summary mode (`CLIENT_SUMMARY_SECS`, `CLIENT_RUN_SECS`), added for
  this step and covered by unit tests in `access-client/src/summary.rs`.

## Step checklist

- [x] Camera frames arrive at 30 fps for at least 60 s through the full path (30.0 fps over 70 s)
- [x] Microphone frames arrive at 16 kHz for at least 60 s through the full path (16 013 Hz over 70 s)
- [x] Dropped frame count and latency recorded in a dated results note (0 dropped; latency per 10 s window)
- [x] Tests still run without hardware (the real-camera and real-microphone tests are `#[ignore]`)
- [ ] Cross-host run (producer and hub on different machines) — not possible on one laptop; record it here when done

## Stage 6 definition of done

- [x] Camera capture at 30 fps for at least 60 s through `device-node` → `control-hub` → `access-client`
- [x] Microphone capture at 16 kHz for at least 60 s through the same path
- [x] Dropped frame count and latency recorded in a dated results note
- [x] Tests still hardware-free
