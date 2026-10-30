# Hardware run — 2026-10-30, Framework Laptop 12

Results for [`docs/specs/06-hardware-validation/step-12-hardware-validation.md`](../specs/06-hardware-validation/step-12-hardware-validation.md).
Started at `3c3d183`, built in the `devenv` shell (rustc 1.99.0). The device-side findings below led to steps 19 and
20; the end-to-end run at the bottom was made after those landed.

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

`AlsaMicrophoneDevice::get_all_alsa_microphones` dropped `default`, `null`, `pipewire` and `sysdefault` by base
name and then dropped every hint containing `DEV`, which removed `front:CARD=PCH,DEV=0` too. On a PipeWire desktop
that left nothing. Fixed in step 20 (`select_capture_hints`: one converting plugin per card, plus sound-server PCMs).
`device-node list` now reports:

```
num microphones: 2
Microphone: "sysdefault:CARD=PCH" -- HDA Intel PCH, ALC285 Analog
Microphone: "pipewire" -- PipeWire Sound Server
```

Ignored capture test (`cargo test -p devices --test microphone-stream-test -- --ignored --nocapture`), 3 s each at
16 kHz mono, 512-sample packets:

| PCM | First packet | Packets | Delivered rate | Notes |
|---|---|---|---|---|
| `sysdefault:CARD=PCH` | 126 ms | 91 | 15 999 Hz | `shape [512, 1]`; may be busy while another client holds the card |
| `pipewire` | 205 ms | 90 | 16 179 Hz | works while PipeWire owns the card |

The samples are heavily clipped (RMS 0.89 of full scale, 40–70 % of samples at ±32767). This is the machine, not the
capture code: `arecord -D pipewire -f S16_LE -r 16000 -c 1` records the same clipping (RMS 0.82), and `amixer`
shows `Capture` at 100 % (+30 dB) with `Internal Mic Boost` at 100 % (+30 dB). The capture loop itself is faithful.

Found on the way: the microphone header carries `frame_shape = [buffer_size, channels]` but not the sample rate,
and `start()` asks ALSA for the nearest rate. A consumer has to assume the node's configured rate. Tracked as step 21.

## Python client

Separate from the hardware run: `pip install ./python-client` into a fresh venv followed by `pytest` passed (1 test,
4.5 s), which closes the open clean-virtualenv item for stage 4.

## End-to-end run (after steps 19 and 20)

Three-process topology on one host, 70 s, workspace after `db8914b` plus the `access-client` summary mode:

```
control-hub   HUB_PRODUCERS=127.0.0.1:5555,127.0.0.1:5557 HUB_STATUS_ADDR=127.0.0.1:8081
device-node   run camera 0 127.0.0.1 5555   (DEVICE_NODE_STATUS_ADDR=127.0.0.1:8083, default selection → 1920×1080 MJPG 1/30)
device-node   run mic 1 127.0.0.1 5557      (DEVICE_NODE_STATUS_ADDR=127.0.0.1:8082, PCM "pipewire", 16 kHz mono, 512 samples)
access-client CLIENT_SUMMARY_SECS=10 CLIENT_RUN_SECS=70
```

Hub status at t+20 s: `{"producers":2,"consumers":1}`; both nodes `{"device":"Running","connection":"Active"}`.

Client totals over 70.0 s (latency is `rx_timestamp - tx_timestamp` on the client, both stamps from the same host clock;
the client polls every 5 ms, which is part of the measured latency):

| Device | Frames | Rate | Expected | Dropped | Latency mean (per 10 s window) | Latency max | Bandwidth |
|---|---|---|---|---|---|---|---|
| `/dev/video0`, 1920×1080 MJPG | 2100 | 30.0 fps | 30 fps | 0 | 21.7–27.7 ms | 77.5 ms (one window; others ≤ 43 ms) | ~995 Mbit/s |
| `pipewire`, 512-sample packets | 2188 | 31.3 pkt/s = 16 013 Hz | 16 000 Hz | 0 | 4.0–6.7 ms | 30.2 ms | 0.3 Mbit/s |

Every 10 s window delivered 300 or 301 camera frames and 311–316 microphone packets. `dropped_total` (receiver
sequence gaps across all devices) stayed at 0 for the whole run.

Notes:

- This camera's MJPG at 1080p is close to 1 Gbit/s (~415 KB per frame), so a real LAN deployment at this mode needs
  gigabit links or a smaller mode (`DEVICE_NODE_CAMERA_WIDTH=1280 …`).
- A first attempt failed because `DEVICE_NODE_STATUS_ADDR`'s default `127.0.0.1:8080` was already taken by an unrelated
  service on this host; the node exits with `failed to bind status server … Address already in use`, as intended.
- The 60 s acceptance criterion for step 12 is met for both devices. Latency numbers are single-host and say nothing
  about cross-host clock skew.

## Sync run (step 10, after the Aligner changes)

Same topology plus `HUB_SYNC_PORT=5558 HUB_SYNC_DEVICES=/dev/video0,pipewire HUB_SYNC_TOLERANCE_MS=20`; a raw client
and an aligned client (`CLIENT_ALIGNED_PORT=5558`) read in parallel for 70 s.

First attempt, with the sync code as it was: 656 groups in 70 s (9.4/s) while the raw stream carried every frame.
Two causes, both visible only on real devices:

| Cause | Evidence | Fix |
|---|---|---|
| Frames were aligned on `tx_timestamp`, the send time | The microphone sends four 512-sample packets back to back every 128 ms (PipeWire's 2048-sample quantum): `tx` gaps of 0.0, 0.0, 0.0, 128 ms. All four carried the same time, so only one camera frame per 128 ms could pair | New header field `capture_timestamp`: the camera uses the driver's monotonic buffer timestamp, the microphone subtracts the frames still queued in ALSA (`avail / rate`) from the read time. Sync aligns on it and falls back to `tx_timestamp` |
| `Aligner` dropped a sample as "stale" once anything newer than `time + tolerance` arrived | With 20 ms tolerance, a partner that is 25 ms later in transport is discarded before it can pair | `max_wait` (default 200 ms, `HUB_SYNC_MAX_WAIT_MS`) bounds the wait; an anchor is dropped immediately only when every other device has already moved past its window |

Second run, 70 s:

| Stream | Groups / frames | Rate | Skew inside a group | Latency tx→rx | Age capture→rx |
|---|---|---|---|---|---|
| Aligned (`/dev/video0` + `pipewire`) | 2101 groups, one per camera frame | 30.0/s | mean 8.3 ms, max 17.9 ms | mean 45 ms, max 115 ms | — |
| Raw `/dev/video0` | 2101 frames | 30.0 fps | — | mean 25.6 ms | mean 57 ms, max 92 ms |
| Raw `pipewire` | 2188 packets | 31.3/s | — | mean 4.9 ms | mean 62 ms, max 128 ms |

The aligned stream's extra latency is the wait for the partner device, bounded by the microphone's 128 ms quantum.
A smaller PipeWire quantum or a larger microphone buffer would lower it; the alignment itself adds nothing further.
