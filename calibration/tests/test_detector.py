import pathlib

import cv2
import numpy as np
import pytest

from calibration import (
    CORNER_COUNT,
    calibrate_intrinsics,
    decode_payload,
    default_board,
    detect_reference_points,
)
from calibration.detector import ReferencePoints

FIXTURE = pathlib.Path(__file__).parent / "fixtures" / "charuco.png"
SQUARE_PX, MARGIN_PX = 100, 50  # as rendered by make_fixture.py
COLS = 6  # inner corners per row


def expected_corner(corner_id: int) -> tuple[float, float]:
    """Pixel position of an inner corner in the fixture, from the rendering geometry alone."""
    row, col = divmod(corner_id, COLS)
    return (MARGIN_PX + (col + 1) * SQUARE_PX, MARGIN_PX + (row + 1) * SQUARE_PX)


@pytest.fixture(scope="module")
def fixture_image():
    image = cv2.imread(str(FIXTURE))
    assert image is not None, "fixture image is missing"
    return image


def test_fixture_returns_every_expected_reference_point(fixture_image):
    found = detect_reference_points(fixture_image)
    assert found.ids.tolist() == list(range(CORNER_COUNT))
    expected = np.array([expected_corner(i) for i in found.ids], dtype=np.float32)
    assert np.abs(found.points - expected).max() < 0.5


def test_detection_is_deterministic(fixture_image):
    a = detect_reference_points(fixture_image)
    b = detect_reference_points(fixture_image)
    assert np.array_equal(a.ids, b.ids) and np.array_equal(a.points, b.points)


def test_partial_view_still_gives_identified_points(fixture_image):
    cropped = fixture_image.copy()
    cropped[:, 450:] = 255  # hide the right part of the board
    found = detect_reference_points(cropped)
    assert 0 < len(found) < CORNER_COUNT
    for corner_id, point in zip(found.ids, found.points):
        assert np.abs(point - expected_corner(corner_id)).max() < 0.5


def test_blank_image_has_no_points():
    found = detect_reference_points(np.full((480, 640, 3), 255, np.uint8))
    assert len(found) == 0


def test_mjpg_payload_decodes_and_detects(fixture_image):
    ok, jpeg = cv2.imencode(".jpg", fixture_image, [cv2.IMWRITE_JPEG_QUALITY, 95])
    assert ok
    decoded = decode_payload(jpeg.tobytes(), "MJPG", [800, 600])
    found = detect_reference_points(decoded)
    assert len(found) == CORNER_COUNT


def test_yuyv_payload_decodes_and_detects(fixture_image):
    gray = cv2.cvtColor(fixture_image, cv2.COLOR_BGR2GRAY)
    yuyv = np.stack([gray, np.full_like(gray, 128)], axis=-1)  # Y0 U Y1 V with neutral chroma
    decoded = decode_payload(yuyv.tobytes(), "YUYV", [800, 600])
    assert len(detect_reference_points(decoded)) == CORNER_COUNT


def test_unknown_pixel_format_is_an_error():
    with pytest.raises(ValueError):
        decode_payload(b"", "H264", [1, 1])


def test_intrinsics_are_recovered_from_projected_views():
    board = default_board()
    corners3d = board.getChessboardCorners().astype(np.float32)
    true_k = np.array([[800, 0, 320], [0, 800, 240], [0, 0, 1]], np.float64)
    size = (640, 480)
    views = []
    poses = [
        ((0.1, -0.2, 0.0), (-0.12, -0.08, 0.6)),
        ((-0.3, 0.1, 0.1), (-0.1, -0.05, 0.7)),
        ((0.2, 0.3, -0.1), (-0.14, -0.1, 0.65)),
        ((-0.1, -0.35, 0.05), (-0.1, -0.06, 0.55)),
        ((0.35, 0.05, 0.15), (-0.12, -0.09, 0.75)),
    ]
    for rvec, tvec in poses:
        pts, _ = cv2.projectPoints(
            corners3d, np.array(rvec), np.array(tvec), true_k, np.zeros(5)
        )
        views.append(
            ReferencePoints(
                np.arange(len(corners3d), dtype=np.int32),
                pts.reshape(-1, 2).astype(np.float32),
            )
        )
    rms, matrix, _ = calibrate_intrinsics(views, size)
    assert rms < 0.5
    assert abs(matrix[0, 0] - 800) < 20 and abs(matrix[1, 1] - 800) < 20


def test_too_few_views_is_an_error():
    with pytest.raises(ValueError):
        calibrate_intrinsics([], (640, 480))
