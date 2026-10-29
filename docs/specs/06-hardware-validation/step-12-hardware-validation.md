# Step 12: Hardware validation of the capture loops

- **Status:** ❌ Open
- **Stage:** 6 — Hardware validation
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 12

## Scope

Run the `V4lCameraDevice` and `AlsaMicrophoneDevice` capture loops against real devices, end to end through `device-node` → `control-hub` → `access-client`.

Acceptance: frames arrive at the configured rate (30 fps camera, 16 kHz microphone) for at least 60 s. The dropped frame count and latency are recorded in a dated results note. Tests stay hardware-free.

## Evidence

- No hardware results note exists in the repository.
- The README lists hardware validation as not yet done.

## Step checklist

- [ ] Camera frames arrive at 30 fps for at least 60 s through the full path
- [ ] Microphone frames arrive at 16 kHz for at least 60 s through the full path
- [ ] Dropped frame count and latency recorded in a dated results note
- [ ] Tests still run without hardware

## Stage 6 definition of done

- [ ] Camera capture at 30 fps for at least 60 s through `device-node` → `control-hub` → `access-client`
- [ ] Microphone capture at 16 kHz for at least 60 s through the same path
- [ ] Dropped frame count and latency recorded in a dated results note
- [ ] Tests still hardware-free
