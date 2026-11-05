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
3. Write the result as a calibration file: per camera the device id, image size, camera matrix, distortion, and
   rotation and translation to the reference frame. The recorder (step 24) copies this file into each session.

Acceptance: on synthetic views of the board projected through two or more known camera poses, the recovered
relative poses match within 1 mm and 0.1°. A real two-camera run is recorded in `docs/hardware/`.

## Step checklist

- [ ] Per-group board pose per camera
- [ ] Pose chaining to a reference camera and joint refinement
- [ ] Calibration file format defined and written (documented in `calibration/README.md`)
- [ ] Synthetic multi-camera test recovers known poses
- [ ] Real two-camera run recorded in `docs/hardware/`
