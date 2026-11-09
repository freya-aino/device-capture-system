//! Reference-point detection (ChArUco inner corners with stable ids) and intrinsics estimation.

use anyhow::{Result, ensure};
use opencv::calib3d;
use opencv::core::{Mat, Point2f, Point3f, Size, TermCriteria, Vector};
use opencv::objdetect::CharucoDetector;
use opencv::prelude::*;

use crate::board::default_board;

/// Detected board corners. `points[i]` (pixels, x then y) is the corner with board id `ids[i]`.
/// Sorted by id, so equal inputs give equal outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferencePoints {
    pub ids: Vec<i32>,
    pub points: Vec<[f32; 2]>,
}

impl ReferencePoints {
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

/// Detects ChArUco corners in a BGR or grayscale image. Partial views are fine.
pub fn detect_reference_points(image: &Mat) -> Result<ReferencePoints> {
    let detector = CharucoDetector::new_def(&default_board()?)?;
    let mut corners = Vector::<Point2f>::new();
    let mut ids = Vector::<i32>::new();
    detector.detect_board_def(image, &mut corners, &mut ids)?;
    let mut pairs: Vec<(i32, [f32; 2])> = ids
        .iter()
        .zip(corners.iter())
        .map(|(id, p)| (id, [p.x, p.y]))
        .collect();
    pairs.sort_by_key(|(id, _)| *id);
    Ok(ReferencePoints {
        ids: pairs.iter().map(|(id, _)| *id).collect(),
        points: pairs.iter().map(|(_, p)| *p).collect(),
    })
}

/// A camera's estimated intrinsics.
pub struct Intrinsics {
    /// Root-mean-square reprojection error in pixels.
    pub rms: f64,
    pub camera_matrix: Mat,
    pub dist_coeffs: Mat,
}

/// Estimates one camera's intrinsics from several views of the board. `image_size` is
/// `(width, height)`. Views with fewer than four points are skipped; at least three usable views are
/// required.
pub fn calibrate_intrinsics(
    views: &[ReferencePoints],
    image_size: (i32, i32),
) -> Result<Intrinsics> {
    let board_corners = default_board()?.get_chessboard_corners()?;
    let mut object_points = Vector::<Vector<Point3f>>::new();
    let mut image_points = Vector::<Vector<Point2f>>::new();
    for view in views.iter().filter(|v| v.len() >= 4) {
        let obj = view
            .ids
            .iter()
            .map(|&id| board_corners.get(id as usize))
            .collect::<Result<Vector<Point3f>, _>>()?;
        let img = view
            .points
            .iter()
            .map(|p| Point2f::new(p[0], p[1]))
            .collect();
        object_points.push(obj);
        image_points.push(img);
    }
    ensure!(
        object_points.len() >= 3,
        "need at least 3 views with 4 or more reference points each"
    );
    let mut camera_matrix = Mat::default();
    let mut dist_coeffs = Mat::default();
    let mut rvecs = Vector::<Mat>::new();
    let mut tvecs = Vector::<Mat>::new();
    let rms = calib3d::calibrate_camera(
        &object_points,
        &image_points,
        Size::new(image_size.0, image_size.1),
        &mut camera_matrix,
        &mut dist_coeffs,
        &mut rvecs,
        &mut tvecs,
        0,
        TermCriteria::default()?,
    )?;
    Ok(Intrinsics {
        rms,
        camera_matrix,
        dist_coeffs,
    })
}
