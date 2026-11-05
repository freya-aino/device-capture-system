"""The calibration target: one ChArUco board shared by the detector, the fixture and the tests."""

import cv2

SQUARES_X = 7
SQUARES_Y = 5
SQUARE_LENGTH = 0.04  # metres
MARKER_LENGTH = 0.03  # metres
DICTIONARY = cv2.aruco.DICT_5X5_100


def default_board() -> "cv2.aruco.CharucoBoard":
    dictionary = cv2.aruco.getPredefinedDictionary(DICTIONARY)
    return cv2.aruco.CharucoBoard(
        (SQUARES_X, SQUARES_Y), SQUARE_LENGTH, MARKER_LENGTH, dictionary
    )


# Inner chessboard corners are the reference points: (SQUARES_X - 1) * (SQUARES_Y - 1) of them.
CORNER_COUNT = (SQUARES_X - 1) * (SQUARES_Y - 1)
