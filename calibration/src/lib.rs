//! Camera calibration over a ChArUco board: reference-point detection, intrinsics, multi-camera
//! extrinsics, and decoding of camera frames from the wire. A consumer of the hub's frames; nothing here runs inside `control-hub`.

mod board;
mod detector;
mod extrinsics;
mod frames;

pub use board::{CORNER_COUNT, DICTIONARY_NAME, default_board};
pub use detector::{Intrinsics, ReferencePoints, calibrate_intrinsics, detect_reference_points};
pub use extrinsics::{
    BoardInfo, CALIBRATION_FORMAT, Calibration, CameraCalibration, CameraModel, CameraView,
    ExtrinsicsOptions, GroupViews, Pose, board_corners, board_pose, calibrate_extrinsics,
    calibrate_extrinsics_with, matrix_to_rotvec, rotvec_to_matrix,
};
pub use frames::{decode_camera_frame, decode_payload};
