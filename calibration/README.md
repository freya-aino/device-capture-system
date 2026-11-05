# calibration

Python consumer package for camera calibration ([item 17](../docs/steps/step-17-calibration.md)). It is not a
cargo workspace member and runs outside the hub: it reads frames through
[`python-client`](../python-client/README.md), so OpenCV stays out of the Rust binaries.

- `detect_reference_points(image)` returns the ChArUco inner corners it finds, as ids and pixel positions,
  sorted by id. Partial views work.
- `calibrate_intrinsics(views, (width, height))` returns `(rms, camera_matrix, dist_coeffs)` from at least
  three views.
- `decode_camera_frame(frame)` / `camera_images(ds, device_id)` decode `device_system.Frame` payloads. The
  wire carries the camera's native format: `MJPG` or `YUYV`
  ([`shared/README.md`](../shared/README.md)).

The board (7×5 squares, 40 mm squares, 30 mm markers, `DICT_5X5_100`) is defined once in
`src/calibration/board.py`. Print it at that size, or change the constants and regenerate the fixture.

```python
import device_system
from calibration import camera_images, detect_reference_points, calibrate_intrinsics

ds = device_system.DeviceSystem("127.0.0.1", 5556)
views = []
for image in camera_images(ds, "camera-device-id"):
    points = detect_reference_points(image)
    if len(points) >= 12:
        views.append(points)
    if len(views) == 15:
        break
rms, matrix, dist = calibrate_intrinsics(views, (image.shape[1], image.shape[0]))
```

## Tests

```bash
pip install -e './calibration[test]'
pytest calibration
```

`tests/fixtures/charuco.png` is checked in; `tests/make_fixture.py` regenerates it. The test compares the
detected corners with the positions implied by the rendering geometry (within 0.5 px).

Not done: multi-camera extrinsics, and a run against a real camera and printed board.
