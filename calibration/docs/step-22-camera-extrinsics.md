# Step 22: Camera extrinsics in a shared world frame

- **Status:** ⚠️ Implemented (2026-11-09), hardware run open: the solver and the calibration file are done and meet
  the synthetic acceptance criterion; the real two-camera run needs a second camera
- **Stage:** 7 — Calibration
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 22

## Scope

Follow-up of [step 17](step-17-calibration.md). Estimate every camera's pose relative to one reference camera
from aligned groups in which two or more cameras see the ChArUco board at the same capture time. Downstream
consumers need this to combine views, for example to triangulate keypoints.

1. For each aligned group, detect reference points per camera (`detect_reference_points`) and solve each
   camera's board pose with its intrinsics (`solvePnP`).
2. Chain the pairwise poses to the reference camera and refine them jointly (bundle adjustment over all groups,
   with the intrinsics held fixed).
3. Write the result as a calibration file (below). The recorder (step 24) copies it into each session.

### Calibration file

JSON, units in metres and pixels. The reference camera has the identity rotation and zero translation. A point
`X_ref` in the reference frame maps to camera `c` as `X_c = R_c · X_ref + t_c`.

```json
{
  "format": 1,
  "board": { "squares": [7, 5], "square_m": 0.04, "marker_m": 0.03, "dictionary": "DICT_5X5_100" },
  "reference": "/dev/video0",
  "cameras": [
    {
      "device_id": "/dev/video0",
      "image_size": [1920, 1080],
      "camera_matrix": [[fx, 0, cx], [0, fy, cy], [0, 0, 1]],
      "dist_coeffs": [k1, k2, p1, p2, k3],
      "rotation": [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
      "translation": [0, 0, 0],
      "intrinsics_rms_px": 0.21,
      "extrinsics_rms_px": 0.35
    }
  ]
}
```

Intrinsics are valid for one `image_size`; a camera recorded in another mode needs its own entry or a new
calibration. `format` is bumped on any incompatible change.

Acceptance: on synthetic views of the board projected through two or more known camera poses, the recovered
relative poses match within 1 mm and 0.1°. A real two-camera run is recorded in `docs/hardware/`.

### Implementation notes (2026-11-09)

- `board_pose` uses `solvePnP` with `SOLVEPNP_IPPE` (the board is planar). Views with fewer than
  `ExtrinsicsOptions::min_points` (default 6) are ignored.
- Pairwise relative poses are averaged over all groups in which both cameras saw the board (quaternion mean for the
  rotation), then chained to the reference camera breadth-first. A camera that no chain of co-visible groups
  connects to the reference is an error naming it; nothing is guessed.
- The joint refinement is a dense Levenberg–Marquardt over all camera poses (reference fixed) and one board pose per
  group, with central-difference Jacobians and the intrinsics held fixed. The projection model is OpenCV's
  pinhole with `k1 k2 p1 p2 k3`; more coefficients are rejected. Problems here are small (hundreds of parameters),
  so no sparse solver is needed.
- `extrinsics_rms_px` per camera is the RMS reprojection error of the refined model over that camera's points.
- Intrinsics come in as `CameraModel` (plain numbers, built from `Intrinsics` with
  `CameraModel::from_intrinsics`), one per camera and image size.

## Step checklist

- [x] Per-group board pose per camera (`board_pose`)
- [x] Pose chaining to a reference camera and joint refinement (`calibrate_extrinsics`)
- [x] Calibration file format defined (above)
- [x] Calibration file written by the crate (`Calibration::save` / `load`, `format` checked), documented in
      `calibration/README.md`
- [x] Synthetic multi-camera test recovers known poses (`tests/extrinsics-test.rs`: three cameras, the third
      only ever seen together with the second, partial views; exact projections recover the poses to ~0.1 mm
      and < 0.01°, 0.3 px noise to < 5 mm and 0.5° with the reported RMS near 0.3 px)
- [ ] Real two-camera run recorded in `docs/hardware/` (needs a second camera)
