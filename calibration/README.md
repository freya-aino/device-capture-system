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

## Build requirements

OpenCV (with `pkg-config` and libclang for the bindings) must be installed. `devenv.nix` provides
`pkgs.opencv`; without Nix, install the OpenCV development package for your distribution.

## Tests

`cargo test -p calibration`. `tests/fixtures/charuco.png` is checked in; `cargo run -p calibration --example
make-fixture` regenerates it. The test compares the detected corners with the positions implied by the
rendering geometry (within 0.5 px).

Not done: multi-camera extrinsics ([item 22](docs/step-22-camera-extrinsics.md)), and a run against a real
camera and printed board.

The `opencv` dependency uses the `clang-runtime` feature. Without it, a plain `cargo build --workspace` fails
in OpenCV's build script (`a libclang shared library is not loaded`), because `bindgen` in the `devices`
dependency tree switches `clang-sys` to runtime loading for the whole workspace.
