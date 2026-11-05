# devices

Capture devices that implement `shared::Device`: `V4lCameraDevice` (v4l2) and `AlsaMicrophoneDevice` (ALSA).
Linux only, by decision. There is no Windows module and no `cfg(target_os = "windows")` stubs
([step 18](docs/step-18-windows-microphone.md)).

## Camera

- `start()` disables exposure auto-priority, sets the format before the interval, and reads both back. It
  refuses to run if the driver negotiates anything other than the requested config, so a mismatch is an
  error, never a silent change.
- Frames are the v4l2 buffer exactly as delivered, in the FourCC named by `pixel_format`.
- `get_configs()` lists the advertised modes; `device-node` picks one (see
  [`device-node/README.md`](../device-node/README.md)).
- Streams at the granted rate in every advertised mode on the 2026-10-30 survey
  ([`docs/hardware/`](../docs/hardware/)).

## Microphone

- Enumerates one converting PCM per card plus sound-server PCMs (`select_capture_hints`).
- Captures 16 kHz mono from both on real hardware, and stamps the granted sample rate on every frame.
- `capture_timestamp` is the read time minus the frames still queued in ALSA.
- Defaults: 16 kHz, 1 channel, 512-sample buffer, `S16LE`.

The payload layout of both devices is the wire protocol in [`shared/README.md`](../shared/README.md).

## Build items

[Step 4 (enumeration filters)](docs/step-04-device-enumeration-filters.md),
[Step 8 (camera buffers and format)](docs/step-08-camera-buffers-format.md),
[Step 18 (Windows microphone)](docs/step-18-windows-microphone.md),
[Step 20 (PipeWire enumeration)](docs/step-20-microphone-enumeration-pipewire.md),
[Step 21 (sample rate on the wire)](docs/step-21-microphone-sample-rate-on-the-wire.md).
