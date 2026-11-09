//! Synthetic multi-camera extrinsics: the board projected through known camera poses (with OpenCV's
//! own projectPoints, independent of the solver's projection) must give back those poses.

use calibration::{
    CORNER_COUNT, Calibration, CameraModel, CameraView, ExtrinsicsOptions, GroupViews, Pose,
    ReferencePoints, board_corners, board_pose, calibrate_extrinsics, calibrate_extrinsics_with,
};
use opencv::calib3d;
use opencv::core::{Mat, Point2f, Point3f, Vector, no_array};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn camera(id: &str, k1: f64) -> CameraModel {
    CameraModel {
        device_id: id.into(),
        image_size: (WIDTH, HEIGHT),
        camera_matrix: [[1000.0, 0.0, 960.0], [0.0, 1000.0, 540.0], [0.0, 0.0, 1.0]],
        dist_coeffs: vec![k1, 0.0, 0.0, 0.0, 0.0],
        intrinsics_rms_px: 0.2,
    }
}

/// Deterministic pseudo-random numbers in [-1, 1).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    }

    fn gaussian(&mut self) -> f64 {
        let (u1, u2) = ((self.next() + 1.0) / 2.0, (self.next() + 1.0) / 2.0);
        (-2.0 * u1.max(1e-12).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Board poses in the reference camera's frame: in front of the cameras, tilted a little.
fn board_poses(count: usize) -> Vec<Pose> {
    let mut rng = Lcg(42);
    (0..count)
        .map(|_| {
            Pose::from_rotvec(
                [0.3 * rng.next(), 0.3 * rng.next(), 0.2 * rng.next()],
                [
                    -0.12 + 0.1 * rng.next(),
                    -0.08 + 0.08 * rng.next(),
                    1.1 + 0.2 * rng.next(),
                ],
            )
        })
        .collect()
}

/// Projects the board through OpenCV, keeping the corners that land inside the image.
fn view(
    camera: &CameraModel,
    camera_pose: &Pose,
    board: &Pose,
    noise_px: f64,
    rng: &mut Lcg,
) -> CameraView {
    let to_camera = camera_pose.compose(board);
    let (rotvec, translation) = (to_camera.rotvec(), to_camera.translation);
    let rvec = Mat::from_slice(&rotvec).unwrap();
    let tvec = Mat::from_slice(&translation).unwrap();
    let k = Mat::from_slice_2d(&camera.camera_matrix).unwrap();
    let d = Mat::from_slice(&camera.dist_coeffs).unwrap();
    let object: Vector<Point3f> = board_corners()
        .unwrap()
        .iter()
        .map(|p| Point3f::new(p[0] as f32, p[1] as f32, p[2] as f32))
        .collect();
    let mut projected = Vector::<Point2f>::new();
    calib3d::project_points(
        &object,
        &rvec,
        &tvec,
        &k,
        &d,
        &mut projected,
        &mut no_array(),
        0.0,
    )
    .unwrap();
    let mut ids = Vec::new();
    let mut points = Vec::new();
    for (id, p) in projected.iter().enumerate() {
        let (x, y) = (
            p.x as f64 + noise_px * rng.gaussian(),
            p.y as f64 + noise_px * rng.gaussian(),
        );
        if x >= 0.0 && x < WIDTH as f64 && y >= 0.0 && y < HEIGHT as f64 {
            ids.push(id as i32);
            points.push([x as f32, y as f32]);
        }
    }
    CameraView {
        device_id: camera.device_id.clone(),
        points: ReferencePoints { ids, points },
    }
}

struct Rig {
    cameras: Vec<CameraModel>,
    poses: Vec<Pose>,
}

fn rig() -> Rig {
    Rig {
        cameras: vec![camera("A", 0.0), camera("B", -0.08), camera("C", 0.03)],
        poses: vec![
            Pose::IDENTITY,
            Pose::from_rotvec([0.0, -0.25, 0.01], [0.45, 0.02, 0.06]),
            Pose::from_rotvec([0.08, 0.3, -0.03], [-0.4, 0.1, 0.12]),
        ],
    }
}

/// Groups 0–7: cameras A and B see the board; groups 8–15: B and C. C is never seen with the
/// reference A, so its pose must be chained through B.
fn groups(rig: &Rig, noise_px: f64) -> Vec<GroupViews> {
    let mut rng = Lcg(7);
    board_poses(16)
        .iter()
        .enumerate()
        .map(|(g, board)| {
            let cams: &[usize] = if g < 8 { &[0, 1] } else { &[1, 2] };
            cams.iter()
                .map(|&c| view(&rig.cameras[c], &rig.poses[c], board, noise_px, &mut rng))
                .collect()
        })
        .collect()
}

fn assert_poses_match(calibration: &Calibration, rig: &Rig, mm: f64, deg: f64) {
    assert_eq!(calibration.reference, "A");
    for (model, truth) in rig.cameras.iter().zip(&rig.poses) {
        let cam = calibration
            .camera(&model.device_id)
            .expect("camera in file");
        let got = cam.pose();
        let (dist_mm, angle) = (got.distance_to(truth) * 1000.0, got.angle_to_deg(truth));
        assert!(
            dist_mm < mm && angle < deg,
            "{}: {dist_mm:.3} mm, {angle:.4} deg off (limits {mm} mm, {deg} deg)",
            model.device_id
        );
    }
}

#[test]
fn views_are_partial_but_usable() {
    let rig = rig();
    let groups = groups(&rig, 0.0);
    let counts: Vec<usize> = groups.iter().flatten().map(|v| v.points.len()).collect();
    assert!(counts.iter().all(|&n| n >= 6), "{counts:?}");
    assert!(counts.iter().any(|&n| n == CORNER_COUNT), "{counts:?}");
}

#[test]
fn board_pose_recovers_a_projected_pose() {
    let rig = rig();
    let board = board_poses(1)[0];
    let v = view(&rig.cameras[1], &rig.poses[1], &board, 0.0, &mut Lcg(1));
    let pose = board_pose(&rig.cameras[1], &v.points).unwrap();
    let truth = rig.poses[1].compose(&board);
    assert!(pose.distance_to(&truth) < 1e-4, "{:?}", pose.translation);
    assert!(pose.angle_to_deg(&truth) < 0.01);
}

#[test]
fn exact_projections_give_back_the_poses_within_a_millimetre_and_a_tenth_of_a_degree() {
    let rig = rig();
    let calibration = calibrate_extrinsics(&rig.cameras, &groups(&rig, 0.0), "A").unwrap();
    assert_poses_match(&calibration, &rig, 1.0, 0.1);
    let a = calibration.camera("A").unwrap();
    assert_eq!(a.rotation, Pose::IDENTITY.rotation);
    assert_eq!(a.translation, [0.0; 3]);
    for cam in &calibration.cameras {
        assert!(
            cam.extrinsics_rms_px < 0.01,
            "{}: rms {}",
            cam.device_id,
            cam.extrinsics_rms_px
        );
        assert_eq!(cam.image_size, [WIDTH, HEIGHT]);
        assert_eq!(cam.intrinsics_rms_px, 0.2);
    }
    assert_eq!(calibration.board.squares, [7, 5]);
    assert_eq!(calibration.board.square_m, 0.04);
}

#[test]
fn noisy_projections_stay_close_and_report_their_rms() {
    let rig = rig();
    let calibration = calibrate_extrinsics(&rig.cameras, &groups(&rig, 0.3), "A").unwrap();
    assert_poses_match(&calibration, &rig, 5.0, 0.5);
    for cam in &calibration.cameras {
        assert!(
            (0.15..0.6).contains(&cam.extrinsics_rms_px),
            "{}: rms {}",
            cam.device_id,
            cam.extrinsics_rms_px
        );
    }
}

#[test]
fn file_round_trips_through_disk() {
    let rig = rig();
    let calibration = calibrate_extrinsics(&rig.cameras, &groups(&rig, 0.0), "A").unwrap();
    let dir = std::env::temp_dir().join(format!("calibration-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("calibration.json");
    calibration.save(&path).unwrap();
    let loaded = Calibration::load(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        (loaded.format, &loaded.board, &loaded.reference),
        (1, &calibration.board, &calibration.reference)
    );
    assert_eq!(loaded.cameras.len(), calibration.cameras.len());
    for (a, b) in loaded.cameras.iter().zip(&calibration.cameras) {
        // JSON floats round-trip to the last digit or so, not bit for bit.
        assert_eq!(
            (&a.device_id, a.image_size, a.camera_matrix),
            (&b.device_id, b.image_size, b.camera_matrix)
        );
        assert!(a.pose().distance_to(&b.pose()) < 1e-9);
        assert!(a.pose().angle_to_deg(&b.pose()) < 1e-9);
        assert!((a.extrinsics_rms_px - b.extrinsics_rms_px).abs() < 1e-12);
    }
}

#[test]
fn unconnected_cameras_and_unknown_references_are_errors() {
    let rig = rig();
    let groups = groups(&rig, 0.0);
    let mut cameras = rig.cameras.clone();
    cameras.push(camera("D", 0.0));
    let err = calibrate_extrinsics(&cameras, &groups, "A").unwrap_err();
    assert!(err.to_string().contains("D"), "{err:#}");
    let err = calibrate_extrinsics(&rig.cameras, &groups, "Z").unwrap_err();
    assert!(err.to_string().contains("reference"), "{err:#}");
    // Without the B/C groups, C is unconnected.
    let err = calibrate_extrinsics(&rig.cameras, &groups[..8], "A").unwrap_err();
    assert!(err.to_string().contains("C"), "{err:#}");
    // Too strict a point threshold leaves no usable view.
    let strict = ExtrinsicsOptions {
        min_points: CORNER_COUNT + 1,
        ..ExtrinsicsOptions::default()
    };
    assert!(calibrate_extrinsics_with(&rig.cameras, &groups, "A", &strict).is_err());
}
