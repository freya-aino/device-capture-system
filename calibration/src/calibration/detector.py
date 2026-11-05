"""Reference-point detection: ChArUco inner corners with stable ids."""

from dataclasses import dataclass

import cv2
import numpy as np

from .board import default_board


@dataclass(frozen=True)
class ReferencePoints:
    """Detected board corners. `ids[i]` is the board corner id of `points[i]` (pixels, x then y).

    Sorted by id, so equal inputs give equal outputs.
    """

    ids: np.ndarray  # (N,) int32
    points: np.ndarray  # (N, 2) float32

    def __len__(self) -> int:
        return len(self.ids)


def detect_reference_points(image: np.ndarray, board=None) -> ReferencePoints:
    """Detect ChArUco corners in a BGR or grayscale image. Partial views are fine."""
    board = board or default_board()
    detector = cv2.aruco.CharucoDetector(board)
    corners, ids, _, _ = detector.detectBoard(image)
    if ids is None or len(ids) == 0:
        return ReferencePoints(np.empty(0, np.int32), np.empty((0, 2), np.float32))
    ids = ids.reshape(-1).astype(np.int32)
    points = corners.reshape(-1, 2).astype(np.float32)
    order = np.argsort(ids)
    return ReferencePoints(ids[order], points[order])


def calibrate_intrinsics(views, image_size, board=None):
    """Estimate one camera's intrinsics from `ReferencePoints` of several views of the board.

    Returns `(rms, camera_matrix, dist_coeffs)`. `image_size` is `(width, height)`. Views with fewer
    than four points are skipped; at least three usable views are required.
    """
    board = board or default_board()
    object_points, image_points = [], []
    for view in views:
        if len(view) < 4:
            continue
        obj, img = board.matchImagePoints(
            view.points.reshape(-1, 1, 2), view.ids.reshape(-1, 1)
        )
        object_points.append(obj)
        image_points.append(img)
    if len(object_points) < 3:
        raise ValueError("need at least 3 views with 4 or more reference points each")
    rms, matrix, dist, _, _ = cv2.calibrateCamera(
        object_points, image_points, tuple(image_size), None, None
    )
    return rms, matrix, dist
