# calibration

Native Rust crate for camera calibration ([item 17](docs/step-17-calibration.md)), built on the
`opencv` crate. It is a consumer of the hub's frames and does not run inside `control-hub`, so the hub binary
does not link OpenCV. It depends on `shared` only; feed it frames from `access-client`.

- `detect_reference_points(&Mat)` returns the ChArUco inner corners it finds, as ids and pixel positions,
  sorted by id. Partial views work.
- `calibrate_intrinsics(&[ReferencePoints], (width, height))` returns `Intrinsics { rms, camera_matrix,
  dist_coeffs }` from at least three views.
- `decode_camera_frame(&FramePacket)` / `decode_payload` turn the wire payload into a BGR `Mat`. The wire
  carries the camera's native format: `MJPG` or `YUYV` ([`shared/README.md`](../shared/README.md)).
- `calibrate_extrinsics(&[CameraModel], &[GroupViews], reference_id)` estimates every camera's pose relative to
  the reference from aligned groups in which two or more cameras saw the board, refines all poses jointly with
  the intrinsics fixed, and returns a `Calibration` ([item 22](docs/step-22-camera-extrinsics.md)).
  `Calibration::save` writes the JSON file the recorder copies into sessions (`RECORD_CALIBRATION`).

The board (7×5 squares, 40 mm squares, 30 mm markers, `DICT_5X5_100`) is defined once in `src/board.rs`.
Print it at that size, or change the constants and regenerate the fixture.

```rust
let mut client = AccessClient::connect(/* hub address */)?;
let mut views = Vec::new();
while views.len() < 15 {
    let packet = client.next_frame()?;
    if packet.frame_info.device_info.id != camera_id { continue; }
    let points = detect_reference_points(&decode_camera_frame(&packet)?)?;
    if points.len() >= 12 { views.push(points); }
}
let intrinsics = calibrate_intrinsics(&views, (width, height))?;
```

Extrinsics, from an `AlignedClient` or the hub's sync port:

```rust
let models = vec![CameraModel::from_intrinsics("/dev/video0", (w, h), &intrinsics_0)?, /* one per camera */];
let mut groups: Vec<GroupViews> = Vec::new();
while groups.len() < 20 {
    let group = aligned.next_group(Duration::from_secs(5))?.context("no aligned group")?;
    let views = group.samples.iter()
        .map(|s| Ok(CameraView {
            device_id: s.device_id.clone(),
            points: detect_reference_points(&decode_camera_frame(&s.payload)?)?,
        }))
        .collect::<Result<Vec<_>>>()?;
    if views.iter().filter(|v| v.points.len() >= 6).count() >= 2 { groups.push(views); }
}
calibrate_extrinsics(&models, &groups, "/dev/video0")?.save("calibration.json")?;
```

The file maps a point in the reference camera's frame to camera `c` as `X_c = R_c · X_ref + t_c`; the reference
has identity rotation and zero translation. Every camera must be connected to the reference through groups in
which both cameras of a pair saw the board, directly or via other cameras.

## Build requirements

OpenCV (with `pkg-config` and libclang for the bindings) must be installed. `devenv.nix` provides
`pkgs.opencv`; without Nix, install the OpenCV development package for your distribution.

## Tests

`cargo test -p calibration`. `tests/fixtures/charuco.png` is checked in; `cargo run -p calibration --example
make-fixture` regenerates it. The test compares the detected corners with the positions implied by the
rendering geometry (within 0.5 px).

`tests/extrinsics-test.rs` projects the board through three known camera poses with OpenCV's `projectPoints` and
checks that `calibrate_extrinsics` gets them back (exact input: ~0.1 mm, < 0.01°; 0.3 px noise: < 5 mm, 0.5°).

Not done: a run against real cameras and a printed board (intrinsics on one camera, extrinsics on two).

The `opencv` dependency uses the `clang-runtime` feature. Without it, a plain `cargo build --workspace` fails
in OpenCV's build script (`a libclang shared library is not loaded`), because `bindgen` in the `devices`
dependency tree switches `clang-sys` to runtime loading for the whole workspace.
