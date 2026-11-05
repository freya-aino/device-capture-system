# Step 17: Calibration reference-point model and python-opencv port

- **Status:** ✅ Done (2026-11-05; acceptance met, follow-ups below)
- **Stage:** 7 — Calibration
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 17

## Scope

Choose a general reference-point detection model, then port the python-opencv calibration code.

Acceptance: a deterministic test on a checked-in fixture image returns the expected reference points.

## Decisions (2026-11-05)

- **Starting point:** the python-opencv code to port does not exist in this repository or its history, so the
  calibration is written from scratch rather than ported.
- **Target:** a ChArUco board (7×5 squares, `DICT_5X5_100`, `calibration/src/calibration/board.py`). Partial
  views still give identified points, which suits multi-camera rigs.
- **Placement:** a Python consumer in [`calibration/`](../../calibration/README.md), not inside `control-hub`.
  OpenCV stays out of the hub binary.

## Delivered

- `detect_reference_points`: ChArUco inner corners with stable ids, sorted by id.
- `calibrate_intrinsics`: camera matrix and distortion from several detected views.
- `decode_camera_frame` / `camera_images`: turn `device_system.Frame` payloads (`MJPG`, `YUYV`) into images.
- Checked-in fixture `calibration/tests/fixtures/charuco.png` (rendered by `tests/make_fixture.py`) and
  deterministic tests (`calibration/tests/test_detector.py`).

## Not delivered (follow-ups)

- Multi-camera extrinsics from aligned groups. The Python bindings do not expose aligned groups yet
  (`python-client/README.md`), so this needs either that binding or consumer-side pairing by capture time.
- A run against a real camera and a printed board. The tests use a rendered board and projected points only.

## Evidence

- `calibration/tests/test_detector.py`: 9 tests pass. The fixture returns all 24 corner ids with positions
  within 0.5 px of the rendering geometry; MJPG and YUYV payloads decode and detect; projected views recover
  the focal length.

## Step checklist

- [x] Reference-point detection model chosen (ChArUco)
- [x] Reference-point detection model implemented
- [x] python-opencv calibration code ported (none existed; written from scratch, see Decisions)
- [x] Checked-in fixture image added
- [x] Deterministic test returns the expected reference points on the fixture
- [x] README calibration TODO entries checked off (extrinsics and real-camera run left open as sub-items)

## Stage 7 definition of done

- [x] Reference-point detection model chosen and implemented
- [x] python-opencv calibration code ported (see Decisions)
- [x] Checked-in fixture image with a deterministic test returning the expected reference points
