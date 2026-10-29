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
one. Diagnosed the same day with a per-mode survey (`real_camera_rate_survey`, timing from the first frame):

| Cause | Evidence | Fix (step 19) |
|---|---|---|
| Stream start-up counted against the rate | first frame arrives ~1.0 s after `start()` in every mode | measure from the first frame; the 60 s acceptance run should ignore the first second |
| `exposure_dynamic_framerate` (`V4L2_CID_EXPOSURE_AUTO_PRIORITY`) was on | every mode, 60 fps or 30 fps, MJPG or YUYV, delivered a flat 20.0 fps with ~50 ms gaps, i.e. auto-exposure stretched the frame time in indoor light | `start()` sets the control to off (best effort, warns if the driver lacks it) |
| Interval set before format | with the control off, a 1/30 request streamed at 60.3 fps: `VIDIOC_S_FMT` reset the interval and the check compared the value returned by `S_PARM`, not the value in effect | `start()` sets the format first, then the interval, and reads the parameters back before comparing |

After the fixes every advertised mode delivered its granted rate, measured over 2 s from the first frame:

| Mode | Delivered | Max gap |
|---|---|---|
| 1920×1080 MJPG 1/60 | 60.2 fps | 21 ms |
| 1920×1080 MJPG 1/30 | 30.0 fps | 37 ms |
| 1280×720 MJPG 1/60, 1/30 | 60.2, 30.0 fps | 20, 36 ms |
| 640×480 MJPG 1/60, 1/30 | 60.2, 30.0 fps | 20, 36 ms |
| 640×360 MJPG 1/60, 1/30 | 60.2, 30.0 fps | 20, 36 ms |
| 640×480 YUYV 1/30 | 30.0 fps | 36 ms |
| 640×360 YUYV 1/30 | 30.0 fps | 36 ms |

The 60 s acceptance run through the full path is still open (step 12).

`device-node run camera 0 127.0.0.1 15555` with the built-in default config fails before publishing anything:

```
Error: Camera negotiated 640x480 YUYV instead of requested 1920x1080 YUYV
```

The device node used the fixed `DEFAULT_CAMERA_CONFIG` (1920×1080 YUYV 30 fps) and never consulted the advertised
list, so it could not start on this camera. Fixed in step 19: with `CameraSelection` the node now prints

```
Selection Nearest { width: 1920, height: 1080, fps: 30.0 } picked CameraConfig { 1920x1080 @ 1/30 s/frame (30.0 fps) MJPG } out of 10 advertised configs
```

and with `DEVICE_NODE_CAMERA_WIDTH=640 DEVICE_NODE_CAMERA_HEIGHT=480` it picks `640x480 @ 1/30 YUYV`, the uncompressed
mode winning the tie.

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
