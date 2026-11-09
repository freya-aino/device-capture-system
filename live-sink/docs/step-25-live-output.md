# Step 25: Live output for other applications

- **Status:** ⚠️ Implemented (2026-11-09), hardware run open: conversion, the V4L2 writer and the forwarding loop
  are done and tested against a loopback hub; the `v4l2loopback` round trip and the webcam run need the kernel
  module, which the development laptop's NixOS kernel does not ship (`modinfo v4l2loopback` fails). Run them on a
  host with the module (see the crate README)
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
- [x] RGB3/GREY → YUYV conversion (`src/convert.rs`; exact pixels and a ±2 round trip through `preprocess`)
- [x] V4L2 output writer (`src/sink.rs`, `VIDIOC_S_FMT` once, `write()` per frame) with the newest-frame policy
      (`src/forward.rs`; loopback tests in `tests/forward-test.rs` cover device selection, format fixing from the
      first frame, skipping and the stream-change error)
- [x] Ignored round-trip test (`frames_round_trip_through_a_v4l2loopback_device`), README setup
- [ ] Webcam run into `v4l2loopback` noted in `docs/hardware/` (needs a host with the module)
