# Step 23: Configurable image pre-processor

- **Status:** ❌ Open
- **Stage:** 9 — Delivery
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 23

## Scope

A pipeline of basic image transforms, configured per camera, that can run either in the hub or in a consumer.
It prepares images for the downstream application and does no analysis.

**Crate.** A new `preprocess` library crate, depending on `shared` only. It is pure Rust, with no OpenCV, so
that the hub can use it without linking OpenCV. When the crate exists, this file moves to `preprocess/docs/`.

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

## Step checklist

- [ ] `preprocess` crate with decode (`MJPG`, `YUYV`) and the transforms above
- [ ] TOML config, validated at start (unknown device ids and invalid parameters are errors)
- [ ] Hub output on `HUB_PREPROCESS_PORT`
- [ ] Consumer-side use in `access-client`
- [ ] `RGB3` and `GREY` documented in `shared/README.md`; variables in the crate READMEs
- [ ] Tests as in the acceptance
