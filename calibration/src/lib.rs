//! Camera calibration over a ChArUco board: reference-point detection, intrinsics, and decoding of
//! camera frames from the wire. A consumer of the hub's frames; nothing here runs inside `control-hub`.

mod board;
mod detector;
mod frames;

pub use board::{CORNER_COUNT, default_board};
pub use detector::{Intrinsics, ReferencePoints, calibrate_intrinsics, detect_reference_points};
pub use frames::{decode_camera_frame, decode_payload};
