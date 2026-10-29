# Hardware run — 2026-10-30, Framework Laptop 12

Partial results for [`docs/specs/06-hardware-validation/step-12-hardware-validation.md`](../specs/06-hardware-validation/step-12-hardware-validation.md).
Workspace at `3c3d183`, built in the `devenv` shell (rustc 1.99.0). Only the device side was exercised; the
`device-node → control-hub → access-client` path was not run because the device node does not start on this camera
(see below).

## Camera: `/dev/video0`, "Framework Laptop 12 Webcam Module"

Advertised configs (printed by `cargo test -p devices --test camera-stream-test -- --ignored --nocapture`):

| Resolution | FourCC | Frame rates |
|---|---|---|
| 1920×1080 | MJPG | 60, 30 |
| 1280×720 | MJPG | 60, 30 |
| 640×480 | MJPG | 60, 30 |
| 640×360 | MJPG | 60, 30 |
| 640×480 | YUYV | 30 |
| 640×360 | YUYV | 30 |

The camera offers no uncompressed (YUYV) mode above 640×480. That is typical for a UVC webcam on USB 2 bandwidth.

Ignored streaming test, config picked by `select_camera_config(&configs, 1920, 1080, 30.0)` (which ignores FourCC and so
picked 1920×1080 MJPG at 30 fps):

| Measure | Value |
|---|---|
| Frames in 3.00 s | 42 |
| Delivered rate | 14.0 fps |
| Requested interval | 1/30, granted 1/30 |

The interval check in `V4lCameraDevice::start` passed, but the delivered rate was less than half of the granted
one. Not diagnosed. Candidates: auto-exposure lengthening the frame time in indoor light, or the first frames being
slow to arrive after `STREAMON`. The 60 s acceptance run was not attempted.

`device-node run camera 0 127.0.0.1 15555` with the built-in default config fails before publishing anything:

```
Error: Camera negotiated 640x480 YUYV instead of requested 1920x1080 YUYV
```

The device node uses the fixed `DEFAULT_CAMERA_CONFIG` (1920×1080 YUYV 30 fps) and never consults the advertised
list, so it cannot start on this camera. Tracked as step 19.

## Microphone: HDA Intel PCH, ALC285 Analog

`device-node list` reports `num microphones: 0`, although the card has one capture PCM (`/proc/asound/pcm`:
`00-00: ALC285 Analog : playback 1 : capture 1`). `arecord -L` lists these PCM hints:

```
null
pipewire
default
sysdefault:CARD=PCH
front:CARD=PCH,DEV=0
```

`AlsaMicrophoneDevice::get_all_alsa_microphones` drops `default`, `null`, `pipewire` and `sysdefault` by base name
and then drops every hint containing `DEV`, which removes `front:CARD=PCH,DEV=0` too. On a PipeWire desktop that
leaves nothing. The ALSA capture loop was therefore not exercised. Tracked as step 20.

## Python client

Separate from the hardware run: `pip install ./python-client` into a fresh venv followed by `pytest` passed (1 test,
4.5 s), which closes the open clean-virtualenv item for stage 4.
