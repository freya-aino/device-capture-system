# Step 17: Calibration reference-point model and python-opencv port

- **Status:** ❌ Open
- **Stage:** 7 — Calibration
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 17

## Scope

Choose a general reference-point detection model, then port the python-opencv calibration code.

Acceptance: a deterministic test on a checked-in fixture image returns the expected reference points.

## Inputs still needed (2026-10-30)

This step cannot start from the repository alone:

- The "python-opencv calibration code" to port is not in this repository or its history. Its location (or a
  copy under `docs/reference/`) is needed before porting.
- The reference-point target is not specified. Checkerboard corners (`cv::findChessboardCorners`), ChArUco
  boards and AprilTags each imply a different detector and a different fixture image. ChArUco is the usual
  choice for multi-camera rigs because partial views still yield identified points; this is a proposal, not a
  decision.
- Where the detector runs: in the hub next to the `AlignedRelay` (it already sees aligned groups), or in a
  consumer on the aligned port. The consumer option keeps OpenCV out of the hub binary and is the suggested
  starting point.

What is in place for it: aligned groups with `capture_timestamp`, `pixel_format` on every camera frame (the
detector must decode MJPG or ask for a `YUYV` mode via `DEVICE_NODE_CAMERA_*`), and the Python bindings, so a
first version can be python-opencv over `DeviceSystem` before any Rust port.

## Evidence

- No calibration or OpenCV code exists in the repository.
- The README calibration TODO entries are unchecked.

## Step checklist

- [ ] Reference-point detection model chosen
- [ ] Reference-point detection model implemented
- [ ] python-opencv calibration code ported
- [ ] Checked-in fixture image added
- [ ] Deterministic test returns the expected reference points on the fixture
- [ ] README calibration TODO entries checked off

## Stage 7 definition of done

- [ ] Reference-point detection model chosen and implemented
- [ ] python-opencv calibration code ported
- [ ] Checked-in fixture image with a deterministic test returning the expected reference points
