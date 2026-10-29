# Step 12: Hardware validation of the capture loops

- **Status:** ⚠️ Partial (device-side run done 2026-10-30; end-to-end run blocked by steps 19 and 20)
- **Stage:** 6 — Hardware validation
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 12

## Scope

Run the `V4lCameraDevice` and `AlsaMicrophoneDevice` capture loops against real devices, end to end through `device-node` → `control-hub` → `access-client`.

Acceptance: frames arrive at the configured rate (30 fps camera, 16 kHz microphone) for at least 60 s. The dropped frame count and latency are recorded in a dated results note. Tests stay hardware-free.

## Evidence

- [`docs/hardware/2026-10-30-framework-12-webcam.md`](../../hardware/2026-10-30-framework-12-webcam.md): the camera
  streams through the ignored `devices` test (42 frames in 3 s, 14 fps at 1920×1080 MJPG with a granted 1/30 interval),
  but `device-node run camera` exits because the fixed default config is not advertised, and `device-node list` finds
  no microphone. The full path was therefore not run.
- Steps [19](../02-producers/step-19-camera-config-negotiation.md) and
  [20](../02-producers/step-20-microphone-enumeration-pipewire.md) must land before the 60 s acceptance run is possible.

## Step checklist

- [ ] Camera frames arrive at 30 fps for at least 60 s through the full path
- [ ] Microphone frames arrive at 16 kHz for at least 60 s through the full path
- [ ] Dropped frame count and latency recorded in a dated results note (note exists; it has device-side numbers only)
- [x] Tests still run without hardware (the real-camera test is `#[ignore]`)

## Stage 6 definition of done

- [ ] Camera capture at 30 fps for at least 60 s through `device-node` → `control-hub` → `access-client`
- [ ] Microphone capture at 16 kHz for at least 60 s through the same path
- [ ] Dropped frame count and latency recorded in a dated results note
- [ ] Tests still hardware-free
