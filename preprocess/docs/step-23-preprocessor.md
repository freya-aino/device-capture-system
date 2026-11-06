# Step 23: Configurable image pre-processor

- **Status:** ✅ Done (2026-11-05; validated on the webcam, single host)
- **Stage:** 9 — Delivery
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 23

## Scope

A pipeline of basic image transforms, configured per camera, that can run either in the hub or in a consumer.
It prepares images for the downstream application and does no analysis.

**Crate.** A new `preprocess` library crate, depending on `shared` only. It is pure Rust, with no OpenCV, so
that the hub can use it without linking OpenCV.

**Transforms**, applied in the configured order:

| Transform | Parameters |
|---|---|
| `rotate` | 90, 180 or 270 degrees |
| `flip` | horizontal or vertical |
| `crop` | x, y, width, height |
| `resize` | width, height |
| `brightness` | additive offset, −255 to 255 |
| `contrast` | multiplicative factor around mid-grey |
| `gamma` | exponent |
| `grayscale` | none |

Input is decoded from the frame's `pixel_format` (`MJPG`, `YUYV`). Output is uncompressed `RGB3` (24-bit RGB)
or `GREY`, written into `pixel_format` with `frame_shape` updated to the output size. Capture timestamp, device
info and sequence are kept, so processed frames still align. This adds two `pixel_format` values but changes no
wire struct.

**Configuration.** A TOML file, one pipeline per device id, named by `PREPROCESS_CONFIG`:

```toml
[[device]]
id = "camera-0"
steps = [
  { rotate = 90 },
  { brightness = 10 },
  { contrast = 1.2 },
]
```

**Placement.**
- **Hub:** when `HUB_PREPROCESS_PORT` and `PREPROCESS_CONFIG` are set, `control-hub` publishes processed frames
  on that separate port. The raw and aligned outputs are unchanged, following the `HUB_SYNC_PORT` pattern.
- **Consumer:** `access-client` (`CLIENT_PREPROCESS_CONFIG`) and the recorder and live sink (steps 24, 25)
  apply the same pipeline locally.

**Bandwidth.** Processed frames are uncompressed. A 1920×1080 `RGB3` frame is 6.2 MB, so 30 fps is about
1.5 Gbit/s, which is more than a gigabit LAN carries. The hub output therefore suits loopback consumers, or
pipelines that `crop`/`resize` first. Cross-host consumers should take the raw stream and pre-process locally.
Re-encoding processed frames would put a codec in the hub's hot path and is out of scope (`SPECS.md` §9).

Acceptance: unit tests per transform on small synthetic images with exact expected pixels. A config-parsing
test. A hub test in which a processed frame keeps the capture timestamp of its source and has the transformed
shape.

## Evidence

- `preprocess` crate ([README](../README.md)): 21 unit tests (exact pixels per transform, fused value transforms,
  config parsing and validation) and 5 integration tests on a checked-in JPEG.
- `networking::FrameRelay` runs the pipeline in the hub: `networking/tests/frame-relay-test.rs` (transform, skip,
  error handling, and a slow transform skipping to the newest frame).
- `access-client/tests/preprocess-pipeline-test.rs`: the hub layout publishes processed frames with the
  transformed shape and the source's capture timestamp, while the raw stream stays unchanged.
- Webcam run ([results](../../docs/hardware/2026-11-05-preprocessor.md)): 640×480 YUYV processed at 30.0 fps with
  0 drops and 13 ms latency; 1920×1080 MJPG at 25.5 fps and 80 ms, skipping the frames it cannot keep up with.

## Deviations from the proposal

- Config validation rejects invalid parameters, unknown steps and duplicate device ids. Unknown device ids cannot
  be rejected at start, because devices appear when their producers do.
- The hub relay is live: it drains its input and processes the newest frame per device, and it sends each
  payload as one part. Both came out of the webcam run, where the first version fell 8 s behind at 1080p.
- `HUB_PREPROCESS_PORT` alone does nothing; `PREPROCESS_CONFIG` is required with it.
- The relay reads the proxy's internal `inproc://` output, not the producers. The first version subscribed to every
  producer itself, which made each producer send its stream twice. The sync relay had the same problem and was
  changed with it (2026-11-06).

## Step checklist

- [x] `preprocess` crate with decode (`MJPG`, `YUYV`) and the transforms above
- [x] TOML config, validated at start (invalid parameters and duplicate device ids are errors)
- [x] Hub output on `HUB_PREPROCESS_PORT`
- [x] Consumer-side use in `access-client`
- [x] `RGB3` and `GREY` documented in `shared/README.md`; variables in the crate READMEs
- [x] Tests as in the acceptance
