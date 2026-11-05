"""Regenerates tests/fixtures/charuco.png. The checked-in PNG is the test input; run this only if the
board definition in calibration/board.py changes."""

import pathlib

import cv2

from calibration import default_board

SQUARE_PX = 100
MARGIN_PX = 50

if __name__ == "__main__":
    board = default_board()
    size = (7 * SQUARE_PX + 2 * MARGIN_PX, 5 * SQUARE_PX + 2 * MARGIN_PX)
    image = board.generateImage(size, marginSize=MARGIN_PX, borderBits=1)
    out = pathlib.Path(__file__).parent / "fixtures" / "charuco.png"
    cv2.imwrite(str(out), image)
    print(out, size)
