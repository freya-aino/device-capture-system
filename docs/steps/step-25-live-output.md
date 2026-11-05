# Step 25: Live output for other applications

- **Status:** ❌ Open
- **Stage:** 9 — Delivery
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 25

## Scope

Let another application use a selected stream live, with no recording in between.

**Applications built on this project's clients** already have live access: the hub's raw port, the aligned port
(`HUB_SYNC_PORT`) and the pre-processor port (step 23, `HUB_PREPROCESS_PORT`), read with `AccessClient`,
`AlignedClient` or the Python `DeviceSystem`. This step documents that as the primary live interface.

**Applications that only understand a camera device** (OpenCV `VideoCapture`, OBS, browsers, video tools) get a
new consumer binary, `live-sink`, that writes one selected stream into a `v4l2loopback` device. The application
then opens `/dev/videoN` as if it were a local webcam. When the crate exists, this file moves to its `docs/`.

| Variable | Meaning |
|---|---|
| `LIVE_DEVICE` | device id of the stream to forward |
| `LIVE_SOURCE` | `raw` (default) or `preprocessed` |
| `LIVE_SINK` | loopback device path, e.g. `/dev/video10` |
| `LIVE_PREPROCESS_CONFIG` | optional local pre-processor pipeline (step 23) |

The sink writes `MJPG` frames through unchanged and converts uncompressed input to `YUYV`, which every V4L2
client reads. The loopback device's format is set from the first frame. A change of shape or format mid-stream
is an error.

Network video protocols for third-party players (RTSP, HLS, WebRTC) are out of scope (`SPECS.md` §9). Remote
applications use the ZMQ ports.

Acceptance: an `#[ignore]` test (it needs the `v4l2loopback` kernel module) that forwards a loopback
publisher's synthetic stream and reads the same frames back through V4L2. The setup (`modprobe v4l2loopback`) is
in the README.

## Step checklist

- [ ] ZMQ ports documented as the live interface for client-based applications
- [ ] `live-sink` binary writing a selected stream to `v4l2loopback`
- [ ] MJPG passthrough; uncompressed input converted to YUYV
- [ ] Optional local pre-processing
- [ ] Ignored round-trip test and README setup
