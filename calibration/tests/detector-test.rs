use calibration::{
    CORNER_COUNT, ReferencePoints, calibrate_intrinsics, decode_payload, default_board,
    detect_reference_points,
};
use opencv::core::{Mat, Point2f, Rect, Scalar, Vector, no_array};
use opencv::imgcodecs::{IMREAD_COLOR, IMWRITE_JPEG_QUALITY, imencode, imread};
use opencv::imgproc::{COLOR_BGR2GRAY, cvt_color_def};
use opencv::prelude::*;
use opencv::{calib3d, core};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/charuco.png");
const SQUARE_PX: f32 = 100.0; // as rendered by examples/make-fixture.rs
const MARGIN_PX: f32 = 50.0;
const COLS: i32 = 6; // inner corners per row

/// Pixel position of an inner corner in the fixture, from the rendering geometry alone.
fn expected_corner(id: i32) -> [f32; 2] {
    let (row, col) = (id / COLS, id % COLS);
    [
        MARGIN_PX + (col + 1) as f32 * SQUARE_PX,
        MARGIN_PX + (row + 1) as f32 * SQUARE_PX,
    ]
}

fn fixture() -> Mat {
    let image = imread(FIXTURE, IMREAD_COLOR).unwrap();
    assert!(!image.empty(), "fixture image is missing");
    image
}

fn max_error(found: &ReferencePoints) -> f32 {
    found
        .ids
        .iter()
        .zip(&found.points)
        .flat_map(|(&id, p)| {
            let e = expected_corner(id);
            [(p[0] - e[0]).abs(), (p[1] - e[1]).abs()]
        })
        .fold(0.0, f32::max)
}

#[test]
fn fixture_returns_every_expected_reference_point() {
    let found = detect_reference_points(&fixture()).unwrap();
    assert_eq!(found.ids, (0..CORNER_COUNT as i32).collect::<Vec<_>>());
    assert!(max_error(&found) < 0.5, "max error {}", max_error(&found));
}

#[test]
fn detection_is_deterministic() {
    let image = fixture();
    assert_eq!(
        detect_reference_points(&image).unwrap(),
        detect_reference_points(&image).unwrap()
    );
}

#[test]
fn partial_view_still_gives_identified_points() {
    let mut image = fixture();
    let mut right = Mat::roi_mut(&mut image, Rect::new(450, 0, 350, 600)).unwrap();
    right.set_to(&Scalar::all(255.0), &no_array()).unwrap();
    let found = detect_reference_points(&image).unwrap();
    assert!(!found.is_empty() && found.len() < CORNER_COUNT);
    assert!(max_error(&found) < 0.5);
}

#[test]
fn blank_image_has_no_points() {
    let blank = Mat::new_rows_cols_with_default(480, 640, core::CV_8UC3, Scalar::all(255.0)).unwrap();
    assert!(detect_reference_points(&blank).unwrap().is_empty());
}

#[test]
fn mjpg_payload_decodes_and_detects() {
    let mut jpeg = Vector::<u8>::new();
    let params = Vector::<i32>::from_slice(&[IMWRITE_JPEG_QUALITY, 95]);
    imencode(".jpg", &fixture(), &mut jpeg, &params).unwrap();
    let decoded = decode_payload(jpeg.as_slice(), "MJPG", &[800, 600]).unwrap();
    assert_eq!(detect_reference_points(&decoded).unwrap().len(), CORNER_COUNT);
}

#[test]
fn yuyv_payload_decodes_and_detects() {
    let mut gray = Mat::default();
    cvt_color_def(&fixture(), &mut gray, COLOR_BGR2GRAY).unwrap();
    // Y0 U Y1 V with neutral chroma: every pixel is its luma followed by 128.
    let yuyv: Vec<u8> = gray.data_bytes().unwrap().iter().flat_map(|&y| [y, 128]).collect();
    let decoded = decode_payload(&yuyv, "YUYV", &[800, 600]).unwrap();
    assert_eq!(detect_reference_points(&decoded).unwrap().len(), CORNER_COUNT);
}

#[test]
fn bad_payloads_are_errors() {
    assert!(decode_payload(b"", "H264", &[1, 1]).is_err());
    assert!(decode_payload(&[0; 10], "YUYV", &[800, 600]).is_err());
    assert!(decode_payload(b"not a jpeg", "MJPG", &[1, 1]).is_err());
}

#[test]
fn intrinsics_are_recovered_from_projected_views() {
    let corners3d = default_board().unwrap().get_chessboard_corners().unwrap();
    let k = Mat::from_slice_2d(&[[800.0f64, 0.0, 320.0], [0.0, 800.0, 240.0], [0.0, 0.0, 1.0]]).unwrap();
    let poses = [
        ([0.1, -0.2, 0.0], [-0.12, -0.08, 0.6]),
        ([-0.3, 0.1, 0.1], [-0.1, -0.05, 0.7]),
        ([0.2, 0.3, -0.1], [-0.14, -0.1, 0.65]),
        ([-0.1, -0.35, 0.05], [-0.1, -0.06, 0.55]),
        ([0.35, 0.05, 0.15], [-0.12, -0.09, 0.75]),
    ];
    let views: Vec<ReferencePoints> = poses
        .iter()
        .map(|(r, t)| {
            let (rvec, tvec) = (Mat::from_slice(r).unwrap(), Mat::from_slice(t).unwrap());
            let mut projected = Vector::<Point2f>::new();
            calib3d::project_points(
                &corners3d, &rvec, &tvec, &k, &Mat::zeros(5, 1, core::CV_64F).unwrap(),
                &mut projected, &mut no_array(), 0.0,
            )
            .unwrap();
            ReferencePoints {
                ids: (0..corners3d.len() as i32).collect(),
                points: projected.iter().map(|p| [p.x, p.y]).collect(),
            }
        })
        .collect();
    let result = calibrate_intrinsics(&views, (640, 480)).unwrap();
    assert!(result.rms < 0.5, "rms {}", result.rms);
    let (fx, fy) = (
        *result.camera_matrix.at_2d::<f64>(0, 0).unwrap(),
        *result.camera_matrix.at_2d::<f64>(1, 1).unwrap(),
    );
    assert!((fx - 800.0).abs() < 20.0 && (fy - 800.0).abs() < 20.0, "fx {fx} fy {fy}");
}

#[test]
fn too_few_views_is_an_error() {
    assert!(calibrate_intrinsics(&[], (640, 480)).is_err());
}

