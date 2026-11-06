# Step 25: Live output for other applications

- **Status:** ⚠️ Scaffolded (2026-11-06): configuration and the output-format decision are implemented and tested;
  the V4L2 writer is not
- **Stage:** 9 — Delivery
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 25

## Scope

Let another application use a selected stream live, with no recording in between.

### Client-based applications: the hub's ports (done)

Applications built on this project's clients already have live access. This is the primary live interface:

| Port | Content | Rust | Python |
|---|---|---|---|
| `HUB_CONSUMER_PORT` (5556) | raw frames, native format | `AccessClient` | `DeviceSystem` |
| `HUB_SYNC_PORT` | aligned groups | `GroupReceiver`, `AlignedClient` | not yet (item 28) |
| `HUB_PREPROCESS_PORT` | pre-processed frames, `RGB3`/`GREY` | `AccessClient` | `DeviceSystem` |

### Applications that only open a camera: `live-sink`

A consumer binary that writes one selected stream into a `v4l2loopback` device, so OpenCV `VideoCapture`, OBS,
browsers and video tools open `/dev/videoN` as if it were a local webcam.

| Variable | Default | Meaning |
|---|---|---|
| `LIVE_HUB_IP` | `127.0.0.1` | hub address |
| `LIVE_SOURCE` | `raw` | `raw` (5556) or `processed` (5558) |
| `LIVE_HUB_PORT` | per source | override the port |
| `LIVE_DEVICE` | required | device id of the stream to forward |
| `LIVE_SINK` | required | loopback device path, e.g. `/dev/video10` |

Behaviour:

- **Output format**, from the first frame: `MJPG` and `YUYV` pass through unchanged; `RGB3` and `GREY` are
  converted to `YUYV` (BT.601 limited range, the inverse of `preprocess`'s decode; `GREY` with neutral chroma),
  because every V4L2 client reads `YUYV`. An odd width for a converted format is an error.
- The sink sets the loopback device's output format (`VIDIOC_S_FMT`, `V4L2_BUF_TYPE_VIDEO_OUTPUT`) once and writes
  each frame with `write()`. A later frame with a different shape or format is an error that stops the sink.
- **Live, not complete:** like the hub relays, the sink writes the newest frame and skips frames that arrived while
  it was writing, so a slow reader never adds latency.
- No transforms in the sink. Pre-processing lives with the hub; use `LIVE_SOURCE=processed`.

Setup (documented in the crate README):

```bash
sudo modprobe v4l2loopback video_nr=10 card_label="device-capture" exclusive_caps=1
```

Network video protocols for third-party players (RTSP, HLS, WebRTC) are out of scope (`SPECS.md` §9).

## Acceptance

- Unit tests: configuration, output-format decision, RGB/GREY → YUYV conversion (exact pixels, and a round trip
  through `preprocess`'s YUYV decode within ±2 levels).
- An `#[ignore]` test (needs the `v4l2loopback` module) that forwards a loopback publisher's synthetic stream and
  reads the same frames back through V4L2.
- A webcam run into `v4l2loopback`, read by `ffplay` or OpenCV, noted in `docs/hardware/`.

## Step checklist

- [x] Hub ports documented as the live interface for client-based applications
- [x] Crate scaffold with `LIVE_*` configuration, validated at start
- [x] Output-format decision per input format
- [ ] RGB3/GREY → YUYV conversion
- [ ] V4L2 output writer with newest-frame policy
- [ ] Ignored round-trip test, README setup, webcam run noted
