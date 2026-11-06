# preprocess

Configurable basic image transforms, applied per camera, in the hub or in a consumer
([item 23](docs/step-23-preprocessor.md)). Pure Rust with no OpenCV, so `control-hub` uses it without linking
OpenCV. It prepares images for downstream applications and does no analysis.

## Configuration

A TOML file with one pipeline per device id. Steps run in order:

```toml
[[device]]
id = "/dev/video0"
steps = [
  { rotate = 90 },                                          # 90, 180 or 270, clockwise
  { flip = "horizontal" },                                  # or "vertical"
  { crop = { x = 0, y = 0, width = 640, height = 480 } },
  { resize = { width = 320, height = 240 } },               # bilinear
  { brightness = 10 },                                      # -255..=255, added to every channel
  { contrast = 1.2 },                                       # factor around mid-grey
  { gamma = 0.8 },                                          # out = 255 * (in/255)^gamma
  "grayscale",                                              # BT.601 luma, output GREY
]
```

Invalid parameters, unknown steps and a device configured twice are errors when the file is loaded. A crop that
does not fit the frame is an error for that frame, which is dropped.

## Frames

`Preprocessor::process` decodes the payload by its `pixel_format` (`MJPG`, `YUYV`, or the already-processed
`RGB3`/`GREY`), runs the device's pipeline, and returns a packet with `pixel_format` set to `RGB3` or `GREY` and
`frame_shape` set to the output `[width, height]`. Device info and all timestamps are kept, so processed frames
still align with the rest of the stream. Microphone frames and devices without a pipeline return `None`.

## Where it runs

| Placement | Variables | Devices without a pipeline |
|---|---|---|
| Hub, beside the raw and aligned outputs | `HUB_PREPROCESS_PORT`, `PREPROCESS_CONFIG` ([`control-hub/README.md`](../control-hub/README.md)) | not published on the processed port |
| `access-client` CLI | `CLIENT_PREPROCESS_CONFIG` ([`access-client/README.md`](../access-client/README.md)) | passed through unchanged |
| Your own consumer | `PreprocessConfig::load`, `Preprocessor::process` | your choice |

The pre-processor belongs to the hub, not to the producers: device nodes always publish the camera's native frames,
and the hub's `networking::FrameRelay` reads the hub's own fanned-in stream, so producers send each frame once. That relay is live, not a queue: it processes the newest
frame per device and skips the ones that arrived meanwhile, so latency stays bounded and consumers see skipped
frames as drops. A consumer that calls `process` on every frame it receives builds up latency instead, if its
pipeline is slower than the camera.

## Performance

Processed frames are uncompressed: 1920×1080 `RGB3` is 6.2 MB, about 1.5 Gbit/s at 30 fps. That is more than a
gigabit LAN carries, so cross-host consumers should read the raw stream and pre-process locally, or `crop` /
`resize` early in the pipeline.

Measured on a Framework Laptop 12 ([results](../docs/hardware/2026-11-05-preprocessor.md)), with the pipeline
rotate 90, brightness, contrast:

| Camera mode | Processed rate | Latency (mean) | Hub CPU |
|---|---|---|---|
| 640×480 YUYV | 30.0 fps, 0 dropped | 13 ms | 25 % |
| 1920×1080 MJPG | 25.5 fps, the rest skipped | 80 ms | 150 % |

`cargo run --release -p preprocess --example bench -- frame.jpg` times decode and each transform on one image.

## Tests

`cargo test -p preprocess`: exact-pixel unit tests per transform, config parsing and validation, and packet
processing on a checked-in JPEG (`tests/fixtures/two-tone.jpg`). The hub path is covered by
`networking/tests/frame-relay-test.rs` and `access-client/tests/preprocess-pipeline-test.rs`.
