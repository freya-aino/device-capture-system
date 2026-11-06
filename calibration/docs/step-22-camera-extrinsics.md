# Step 22: Camera extrinsics in a shared world frame

- **Status:** ❌ Open
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

## Step checklist

- [ ] Per-group board pose per camera
- [ ] Pose chaining to a reference camera and joint refinement
- [x] Calibration file format defined (above)
- [ ] Calibration file written by the crate, and documented in `calibration/README.md`
- [ ] Synthetic multi-camera test recovers known poses
- [ ] Real two-camera run recorded in `docs/hardware/`
