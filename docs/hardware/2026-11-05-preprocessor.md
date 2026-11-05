# 2026-11-05: Pre-processor on the Framework Laptop 12 webcam

Results for [step 23](../../preprocess/docs/step-23-preprocessor.md). One host: `device-node run camera 0`, then
`control-hub` with `HUB_PREPROCESS_PORT=5558` and a pipeline for `/dev/video0` of `rotate = 90`,
`brightness = 10`, `contrast = 1.2`, then `access-client` with `CLIENT_SUMMARY_SECS=10 CLIENT_RUN_SECS=30` on the
processed port. Release builds.

## First attempt (relay as committed in `aae8cfe`)

1920×1080 MJPG: 11.4 fps processed, latency growing to 8–11 s, 338 dropped in 30 s. The raw stream at the same time:
29.8 fps, 38 ms. Two causes:

- **Chunking.** The relay sent payloads in `HUB_DATA_CHUNK_SIZE` (1 KiB) parts, about 6000 parts per 6.2 MB RGB
  frame. With 4 MiB chunks the rate rose to 23.3 fps, but latency still climbed to 3.6 s.
- **Queueing at the producer.** The relay read slower than 30 fps, so frames waited in the producer's per-subscriber
  PUB queue. A short input queue on the relay did not help: ZMQ stops reading from the network when it is full,
  which moves the backlog to the producer.

Per-stage timing on a 1080p JPEG (`preprocess` bench) before optimising: decode 23 ms, rotate 90 12 ms, contrast
26 ms (per-value floating point), flip 10 ms, resize to half 22 ms.

## Fixes

- The relay drains every waiting frame before each transform and processes only the newest per device; its input
  queue is large (it is drained continuously) and its output queue small. Payloads go out as one part.
- Value transforms run as one fused lookup table; pixel moves are fixed-size copies, quarter turns in tiles;
  JPEG decoding is parallel. After: decode 15 ms, rotate 90 5 ms, contrast 3 ms, flip 5 ms, resize 13 ms.

## Results after the fixes (`81b8dd7`)

| Camera mode | Processed | Latency mean / max | Hub CPU | Raw stream meanwhile |
|---|---|---|---|---|
| 1920×1080 MJPG | 25.5 fps, 134 skipped in 30 s | 80 / 141 ms | 149 % | 29.8 fps |
| 640×480 YUYV | 30.0 fps, 0 dropped | 13 / 22 ms | 25 % | 30.0 fps |

Consumer side (`CLIENT_PREPROCESS_CONFIG`, same pipeline, 1080p MJPG): 22 fps, latency growing to 2.7 s. The CLI
processes every frame it receives, so a pipeline slower than the camera queues frames in its socket. Use the hub
relay, a smaller camera mode, or an early `resize` for live use.
