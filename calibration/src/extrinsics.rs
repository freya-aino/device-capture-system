//! Camera extrinsics in a shared world frame (`docs/step-22-camera-extrinsics.md`): the pose of every
//! camera relative to one reference camera, from aligned groups in which two or more cameras see the
//! board at the same capture time.
//!
//! Pipeline: `board_pose` per camera and group (`solvePnP`), pairwise relative poses averaged over
//! groups and chained to the reference camera, then a joint Levenberg–Marquardt refinement of all
//! camera and board poses with the intrinsics held fixed.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::Path;

use anyhow::{Context, Result, anyhow, ensure};
use opencv::calib3d;
use opencv::core::{Mat, Point2f, Point3f, Vector};
use opencv::prelude::*;
use serde::{Deserialize, Serialize};

use crate::board::{
    DICTIONARY_NAME, MARKER_LENGTH, SQUARE_LENGTH, SQUARES_X, SQUARES_Y, default_board,
};
use crate::detector::{Intrinsics, ReferencePoints};

/// `format` of the calibration file; bumped on any incompatible change.
pub const CALIBRATION_FORMAT: u32 = 1;

/// One camera's intrinsics in plain numbers, valid for one image size.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraModel {
    pub device_id: String,
    /// `(width, height)` in pixels.
    pub image_size: (u32, u32),
    pub camera_matrix: [[f64; 3]; 3],
    /// OpenCV order: `k1, k2, p1, p2, k3` (shorter lists are zero-padded).
    pub dist_coeffs: Vec<f64>,
    pub intrinsics_rms_px: f64,
}

impl CameraModel {
    pub fn from_intrinsics(
        device_id: &str,
        image_size: (u32, u32),
        intrinsics: &Intrinsics,
    ) -> Result<Self> {
        let k = &intrinsics.camera_matrix;
        ensure!(
            k.rows() == 3 && k.cols() == 3,
            "camera matrix must be 3x3, got {}x{}",
            k.rows(),
            k.cols()
        );
        let mut camera_matrix = [[0.0; 3]; 3];
        for (r, row) in camera_matrix.iter_mut().enumerate() {
            for (c, v) in row.iter_mut().enumerate() {
                *v = *k.at_2d::<f64>(r as i32, c as i32)?;
            }
        }
        let d = &intrinsics.dist_coeffs;
        let dist_coeffs: Vec<f64> = (0..d.total())
            .map(|i| d.at::<f64>(i as i32).copied())
            .collect::<opencv::Result<_>>()?;
        Ok(CameraModel {
            device_id: device_id.to_string(),
            image_size,
            camera_matrix,
            dist_coeffs,
            intrinsics_rms_px: intrinsics.rms,
        })
    }

    fn distortion(&self) -> Result<[f64; 5]> {
        ensure!(
            self.dist_coeffs.len() <= 5,
            "{}: {} distortion coefficients, the extrinsics solver supports k1 k2 p1 p2 k3",
            self.device_id,
            self.dist_coeffs.len()
        );
        let mut d = [0.0; 5];
        d[..self.dist_coeffs.len()].copy_from_slice(&self.dist_coeffs);
        Ok(d)
    }

    fn camera_matrix_mat(&self) -> Result<Mat> {
        Ok(Mat::from_slice_2d(&self.camera_matrix)?)
    }

    fn dist_coeffs_mat(&self) -> Result<Mat> {
        Ok(Mat::from_exact_iter(self.distortion()?.into_iter())?)
    }

    /// Projects a point given in camera coordinates (OpenCV pinhole + radial/tangential model).
    fn project(&self, point: [f64; 3], d: &[f64; 5]) -> [f64; 2] {
        let [x, y, z] = point;
        let (xp, yp) = (x / z, y / z);
        let r2 = xp * xp + yp * yp;
        let radial = 1.0 + d[0] * r2 + d[1] * r2 * r2 + d[4] * r2 * r2 * r2;
        let xpp = xp * radial + 2.0 * d[2] * xp * yp + d[3] * (r2 + 2.0 * xp * xp);
        let ypp = yp * radial + d[2] * (r2 + 2.0 * yp * yp) + 2.0 * d[3] * xp * yp;
        let k = &self.camera_matrix;
        [
            k[0][0] * xpp + k[0][1] * ypp + k[0][2],
            k[1][1] * ypp + k[1][2],
        ]
    }
}

/// One camera's detections in one aligned group.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraView {
    pub device_id: String,
    pub points: ReferencePoints,
}

/// The views of one aligned group: at most one per camera.
pub type GroupViews = Vec<CameraView>;

/// A rigid transform `X' = R · X + t`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub rotation: [[f64; 3]; 3],
    pub translation: [f64; 3],
}

impl Pose {
    pub const IDENTITY: Pose = Pose {
        rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [0.0; 3],
    };

    pub fn from_rotvec(rotvec: [f64; 3], translation: [f64; 3]) -> Pose {
        Pose {
            rotation: rotvec_to_matrix(rotvec),
            translation,
        }
    }

    pub fn rotvec(&self) -> [f64; 3] {
        matrix_to_rotvec(self.rotation)
    }

    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let r = &self.rotation;
        [
            r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2] + self.translation[0],
            r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2] + self.translation[1],
            r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2] + self.translation[2],
        ]
    }

    /// `self ∘ other`: applies `other` first, then `self`.
    pub fn compose(&self, other: &Pose) -> Pose {
        let mut rotation = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                rotation[i][j] = (0..3)
                    .map(|k| self.rotation[i][k] * other.rotation[k][j])
                    .sum();
            }
        }
        Pose {
            rotation,
            translation: self.apply(other.translation),
        }
    }

    pub fn inverse(&self) -> Pose {
        let r = &self.rotation;
        let rt = [
            [r[0][0], r[1][0], r[2][0]],
            [r[0][1], r[1][1], r[2][1]],
            [r[0][2], r[1][2], r[2][2]],
        ];
        let t = self.translation;
        Pose {
            rotation: rt,
            translation: [
                -(rt[0][0] * t[0] + rt[0][1] * t[1] + rt[0][2] * t[2]),
                -(rt[1][0] * t[0] + rt[1][1] * t[1] + rt[1][2] * t[2]),
                -(rt[2][0] * t[0] + rt[2][1] * t[1] + rt[2][2] * t[2]),
            ],
        }
    }

    /// Rotation angle between the two poses, in degrees.
    pub fn angle_to_deg(&self, other: &Pose) -> f64 {
        let rel = self.inverse().compose(other);
        norm(matrix_to_rotvec(rel.rotation)).to_degrees()
    }

    /// Distance between the two translations, in the poses' length unit (metres).
    pub fn distance_to(&self, other: &Pose) -> f64 {
        norm(sub(self.translation, other.translation))
    }
}

/// The board pose in one camera's frame from one view: `X_camera = R · X_board + t`. Needs at least
/// four points; the planar IPPE solver is used.
pub fn board_pose(camera: &CameraModel, points: &ReferencePoints) -> Result<Pose> {
    ensure!(
        points.len() >= 4,
        "{}: {} reference points, solvePnP needs at least 4",
        camera.device_id,
        points.len()
    );
    let corners = board_corners()?;
    let object: Vector<Point3f> = points
        .ids
        .iter()
        .map(|&id| {
            let p = corners
                .get(id as usize)
                .ok_or_else(|| anyhow!("reference point id {id} is not on the board"))?;
            Ok(Point3f::new(p[0] as f32, p[1] as f32, p[2] as f32))
        })
        .collect::<Result<_>>()?;
    let image: Vector<Point2f> = points
        .points
        .iter()
        .map(|p| Point2f::new(p[0], p[1]))
        .collect();
    let (mut rvec, mut tvec) = (Mat::default(), Mat::default());
    let ok = calib3d::solve_pnp(
        &object,
        &image,
        &camera.camera_matrix_mat()?,
        &camera.dist_coeffs_mat()?,
        &mut rvec,
        &mut tvec,
        false,
        calib3d::SOLVEPNP_IPPE,
    )?;
    ensure!(ok, "{}: solvePnP found no board pose", camera.device_id);
    let read = |m: &Mat| -> Result<[f64; 3]> {
        Ok([*m.at::<f64>(0)?, *m.at::<f64>(1)?, *m.at::<f64>(2)?])
    };
    Ok(Pose::from_rotvec(read(&rvec)?, read(&tvec)?))
}

/// The board's inner corners in board coordinates (metres).
pub fn board_corners() -> Result<Vec<[f64; 3]>> {
    Ok(default_board()?
        .get_chessboard_corners()?
        .iter()
        .map(|p| [p.x as f64, p.y as f64, p.z as f64])
        .collect())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardInfo {
    pub squares: [u32; 2],
    pub square_m: f64,
    pub marker_m: f64,
    pub dictionary: String,
}

impl BoardInfo {
    pub fn current() -> BoardInfo {
        BoardInfo {
            squares: [SQUARES_X as u32, SQUARES_Y as u32],
            // The board constants are f32; round to micrometres so the file reads 0.04, not
            // 0.03999999910593033.
            square_m: (SQUARE_LENGTH as f64 * 1e6).round() / 1e6,
            marker_m: (MARKER_LENGTH as f64 * 1e6).round() / 1e6,
            dictionary: DICTIONARY_NAME.to_string(),
        }
    }
}

/// One camera in the calibration file. `X_c = rotation · X_ref + translation`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraCalibration {
    pub device_id: String,
    pub image_size: [u32; 2],
    pub camera_matrix: [[f64; 3]; 3],
    pub dist_coeffs: Vec<f64>,
    pub rotation: [[f64; 3]; 3],
    pub translation: [f64; 3],
    pub intrinsics_rms_px: f64,
    pub extrinsics_rms_px: f64,
}

impl CameraCalibration {
    pub fn pose(&self) -> Pose {
        Pose {
            rotation: self.rotation,
            translation: self.translation,
        }
    }
}

/// The calibration file (`format` 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    pub format: u32,
    pub board: BoardInfo,
    pub reference: String,
    pub cameras: Vec<CameraCalibration>,
}

impl Calibration {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("calibration serializes")
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let calibration: Calibration =
            serde_json::from_str(text).context("parsing the calibration file")?;
        ensure!(
            calibration.format == CALIBRATION_FORMAT,
            "calibration format {} is not supported (expected {CALIBRATION_FORMAT})",
            calibration.format
        );
        Ok(calibration)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        std::fs::write(path, self.to_json()).with_context(|| format!("writing {}", path.display()))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::from_json(&text).with_context(|| format!("in {}", path.display()))
    }

    pub fn camera(&self, device_id: &str) -> Option<&CameraCalibration> {
        self.cameras.iter().find(|c| c.device_id == device_id)
    }
}

/// Tuning of `calibrate_extrinsics`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtrinsicsOptions {
    /// Views with fewer reference points are ignored.
    pub min_points: usize,
    /// Levenberg–Marquardt iterations at most.
    pub max_iterations: usize,
}

impl Default for ExtrinsicsOptions {
    fn default() -> Self {
        ExtrinsicsOptions {
            min_points: 6,
            max_iterations: 50,
        }
    }
}

struct Observation {
    group: usize,
    camera: usize,
    /// Board-frame points and their measured pixels.
    points: Vec<([f64; 3], [f64; 2])>,
}

/// Estimates every camera's pose relative to `reference` from aligned groups, refines all poses
/// jointly, and returns the calibration file content. Every camera must be connected to the
/// reference through groups in which both cameras of a pair saw the board.
pub fn calibrate_extrinsics(
    cameras: &[CameraModel],
    groups: &[GroupViews],
    reference: &str,
) -> Result<Calibration> {
    calibrate_extrinsics_with(cameras, groups, reference, &ExtrinsicsOptions::default())
}

pub fn calibrate_extrinsics_with(
    cameras: &[CameraModel],
    groups: &[GroupViews],
    reference: &str,
    options: &ExtrinsicsOptions,
) -> Result<Calibration> {
    ensure!(!cameras.is_empty(), "no cameras");
    let index: HashMap<&str, usize> = cameras
        .iter()
        .enumerate()
        .map(|(i, c)| (c.device_id.as_str(), i))
        .collect();
    ensure!(
        index.len() == cameras.len(),
        "camera device ids are not unique"
    );
    let reference_index = *index
        .get(reference)
        .ok_or_else(|| anyhow!("reference camera {reference:?} is not among the cameras"))?;
    let distortions: Vec<[f64; 5]> = cameras
        .iter()
        .map(CameraModel::distortion)
        .collect::<Result<_>>()?;
    let corners = board_corners()?;

    // 1. Board pose per camera and group, and the observations the refinement will use.
    let mut observations = Vec::new();
    let mut board_poses: Vec<BTreeMap<usize, Pose>> = Vec::new();
    for (g, group) in groups.iter().enumerate() {
        let mut poses = BTreeMap::new();
        let mut seen = HashSet::new();
        for view in group {
            let Some(&c) = index.get(view.device_id.as_str()) else {
                continue;
            };
            ensure!(
                seen.insert(c),
                "group {g} has two views of camera {:?}",
                view.device_id
            );
            if view.points.len() < options.min_points {
                continue;
            }
            poses.insert(c, board_pose(&cameras[c], &view.points)?);
            let points = view
                .points
                .ids
                .iter()
                .zip(&view.points.points)
                .map(|(&id, p)| {
                    let x = corners
                        .get(id as usize)
                        .ok_or_else(|| anyhow!("reference point id {id} is not on the board"))?;
                    Ok((*x, [p[0] as f64, p[1] as f64]))
                })
                .collect::<Result<Vec<_>>>()?;
            observations.push(Observation {
                group: g,
                camera: c,
                points,
            });
        }
        board_poses.push(poses);
    }
    let usable_groups: Vec<usize> = (0..groups.len())
        .filter(|&g| !board_poses[g].is_empty())
        .collect();
    ensure!(
        !usable_groups.is_empty(),
        "no group has a usable view of the board"
    );

    // 2. Pairwise relative poses averaged over groups, chained to the reference.
    let mut pair_poses: HashMap<(usize, usize), Vec<Pose>> = HashMap::new();
    for poses in &board_poses {
        for (&a, pa) in poses {
            for (&b, pb) in poses {
                if a != b {
                    // X_b = P_b · X_board = P_b · P_a⁻¹ · X_a
                    pair_poses
                        .entry((a, b))
                        .or_default()
                        .push(pb.compose(&pa.inverse()));
                }
            }
        }
    }
    let mut camera_poses: Vec<Option<Pose>> = vec![None; cameras.len()];
    camera_poses[reference_index] = Some(Pose::IDENTITY);
    let mut queue = VecDeque::from([reference_index]);
    while let Some(a) = queue.pop_front() {
        let pose_a = camera_poses[a].expect("queued cameras have poses");
        for b in 0..cameras.len() {
            if camera_poses[b].is_some() {
                continue;
            }
            if let Some(estimates) = pair_poses.get(&(a, b)) {
                // X_b = T_ba · X_a = T_ba · T_a · X_ref
                camera_poses[b] = Some(average_pose(estimates).compose(&pose_a));
                queue.push_back(b);
            }
        }
    }
    let unreachable: Vec<&str> = camera_poses
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_none())
        .map(|(i, _)| cameras[i].device_id.as_str())
        .collect();
    ensure!(
        unreachable.is_empty(),
        "no group connects {} to the reference camera {reference:?}",
        unreachable.join(", ")
    );
    let mut camera_poses: Vec<Pose> = camera_poses.into_iter().map(Option::unwrap).collect();

    // 3. Board pose per group in the reference frame, from the reference camera when it saw it.
    let mut group_poses: Vec<Pose> = usable_groups
        .iter()
        .map(|&g| {
            let (&c, pose) = board_poses[g]
                .get_key_value(&reference_index)
                .unwrap_or_else(|| board_poses[g].iter().next().expect("usable group"));
            // X_ref = T_c⁻¹ · X_c = T_c⁻¹ · P_c · X_board
            camera_poses[c].inverse().compose(pose)
        })
        .collect();
    let group_slot: HashMap<usize, usize> = usable_groups
        .iter()
        .enumerate()
        .map(|(slot, &g)| (g, slot))
        .collect();

    // 4. Joint refinement.
    let free_cameras: Vec<usize> = (0..cameras.len())
        .filter(|&c| c != reference_index)
        .collect();
    let mut params = Vec::with_capacity(6 * (free_cameras.len() + group_poses.len()));
    for &c in &free_cameras {
        params.extend(camera_poses[c].rotvec());
        params.extend(camera_poses[c].translation);
    }
    for pose in &group_poses {
        params.extend(pose.rotvec());
        params.extend(pose.translation);
    }
    let residuals = |params: &[f64], out: &mut Vec<f64>| {
        out.clear();
        let pose_of_camera = |c: usize| -> Pose {
            match free_cameras.iter().position(|&f| f == c) {
                Some(i) => pose_from_params(&params[6 * i..6 * i + 6]),
                None => Pose::IDENTITY,
            }
        };
        for obs in &observations {
            let cam = pose_of_camera(obs.camera);
            let slot = free_cameras.len() + group_slot[&obs.group];
            let board = pose_from_params(&params[6 * slot..6 * slot + 6]);
            let to_camera = cam.compose(&board);
            let model = &cameras[obs.camera];
            let d = &distortions[obs.camera];
            for (x, measured) in &obs.points {
                let p = model.project(to_camera.apply(*x), d);
                out.push(p[0] - measured[0]);
                out.push(p[1] - measured[1]);
            }
        }
    };
    levenberg_marquardt(&mut params, &residuals, options.max_iterations)?;

    for (i, &c) in free_cameras.iter().enumerate() {
        camera_poses[c] = pose_from_params(&params[6 * i..6 * i + 6]);
    }
    for (slot, pose) in group_poses.iter_mut().enumerate() {
        let i = free_cameras.len() + slot;
        *pose = pose_from_params(&params[6 * i..6 * i + 6]);
    }

    // 5. Per-camera RMS of the refined model.
    let mut final_residuals = Vec::new();
    residuals(&params, &mut final_residuals);
    let mut sum_sq = vec![0.0; cameras.len()];
    let mut count = vec![0usize; cameras.len()];
    let mut offset = 0;
    for obs in &observations {
        for _ in &obs.points {
            let (dx, dy) = (final_residuals[offset], final_residuals[offset + 1]);
            sum_sq[obs.camera] += dx * dx + dy * dy;
            count[obs.camera] += 1;
            offset += 2;
        }
    }

    Ok(Calibration {
        format: CALIBRATION_FORMAT,
        board: BoardInfo::current(),
        reference: reference.to_string(),
        cameras: cameras
            .iter()
            .enumerate()
            .map(|(c, model)| CameraCalibration {
                device_id: model.device_id.clone(),
                image_size: [model.image_size.0, model.image_size.1],
                camera_matrix: model.camera_matrix,
                dist_coeffs: model.dist_coeffs.clone(),
                rotation: camera_poses[c].rotation,
                translation: camera_poses[c].translation,
                intrinsics_rms_px: model.intrinsics_rms_px,
                extrinsics_rms_px: if count[c] > 0 {
                    (sum_sq[c] / count[c] as f64).sqrt()
                } else {
                    f64::NAN
                },
            })
            .collect(),
    })
}

fn pose_from_params(p: &[f64]) -> Pose {
    Pose::from_rotvec([p[0], p[1], p[2]], [p[3], p[4], p[5]])
}

/// Mean of several estimates of the same pose: quaternion mean for the rotation (signs aligned to
/// the first), arithmetic mean for the translation.
fn average_pose(estimates: &[Pose]) -> Pose {
    let first = rotvec_to_quat(estimates[0].rotvec());
    let mut q = [0.0; 4];
    let mut t = [0.0; 3];
    for pose in estimates {
        let mut qi = rotvec_to_quat(pose.rotvec());
        if (0..4).map(|i| qi[i] * first[i]).sum::<f64>() < 0.0 {
            qi.iter_mut().for_each(|v| *v = -*v);
        }
        for i in 0..4 {
            q[i] += qi[i];
        }
        for i in 0..3 {
            t[i] += pose.translation[i];
        }
    }
    let n = estimates.len() as f64;
    let qn = (0..4).map(|i| q[i] * q[i]).sum::<f64>().sqrt();
    q.iter_mut().for_each(|v| *v /= qn);
    Pose::from_rotvec(quat_to_rotvec(q), [t[0] / n, t[1] / n, t[2] / n])
}

/// Dense Levenberg–Marquardt with central-difference Jacobians. Small problems only (a few hundred
/// parameters); stops when the cost stops improving or `max_iterations` is reached.
fn levenberg_marquardt(
    params: &mut [f64],
    residuals: &impl Fn(&[f64], &mut Vec<f64>),
    max_iterations: usize,
) -> Result<()> {
    let n = params.len();
    let mut r = Vec::new();
    residuals(params, &mut r);
    ensure!(r.len() >= n, "{} residuals for {n} parameters", r.len());
    let mut cost: f64 = r.iter().map(|v| v * v).sum();
    let mut lambda = 1e-3;
    let (mut plus, mut minus) = (Vec::new(), Vec::new());
    let mut trial = vec![0.0; n];
    let mut r_trial = Vec::new();

    for _ in 0..max_iterations {
        // Jacobian, column by column.
        let m = r.len();
        let mut jacobian = vec![0.0; m * n];
        for j in 0..n {
            let h = 1e-6_f64.max(1e-7 * params[j].abs());
            let saved = params[j];
            params[j] = saved + h;
            residuals(params, &mut plus);
            params[j] = saved - h;
            residuals(params, &mut minus);
            params[j] = saved;
            for i in 0..m {
                jacobian[i * n + j] = (plus[i] - minus[i]) / (2.0 * h);
            }
        }
        let mut jtj = vec![0.0; n * n];
        let mut jtr = vec![0.0; n];
        for i in 0..m {
            let row = &jacobian[i * n..(i + 1) * n];
            for a in 0..n {
                jtr[a] += row[a] * r[i];
                for b in a..n {
                    jtj[a * n + b] += row[a] * row[b];
                }
            }
        }
        for a in 0..n {
            for b in 0..a {
                jtj[a * n + b] = jtj[b * n + a];
            }
        }

        let mut improved = false;
        for _ in 0..12 {
            let mut system = jtj.clone();
            for a in 0..n {
                system[a * n + a] += lambda * (jtj[a * n + a].max(1e-12));
            }
            let rhs: Vec<f64> = jtr.iter().map(|v| -v).collect();
            let Some(delta) = solve_dense(system, rhs, n) else {
                lambda *= 10.0;
                continue;
            };
            for j in 0..n {
                trial[j] = params[j] + delta[j];
            }
            residuals(&trial, &mut r_trial);
            let trial_cost: f64 = r_trial.iter().map(|v| v * v).sum();
            if trial_cost < cost {
                let relative = (cost - trial_cost) / cost.max(1e-30);
                params.copy_from_slice(&trial);
                std::mem::swap(&mut r, &mut r_trial);
                cost = trial_cost;
                lambda = (lambda / 3.0).max(1e-12);
                improved = true;
                if relative < 1e-12 || cost < 1e-20 {
                    return Ok(());
                }
                break;
            }
            lambda *= 4.0;
        }
        if !improved {
            return Ok(());
        }
    }
    Ok(())
}

/// Gaussian elimination with partial pivoting; `None` for a singular system.
fn solve_dense(mut a: Vec<f64>, mut b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    for col in 0..n {
        let pivot = (col..n).max_by(|&x, &y| {
            a[x * n + col]
                .abs()
                .partial_cmp(&a[y * n + col].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if a[pivot * n + col].abs() < 1e-300 {
            return None;
        }
        if pivot != col {
            for k in 0..n {
                a.swap(col * n + k, pivot * n + k);
            }
            b.swap(col, pivot);
        }
        for row in col + 1..n {
            let f = a[row * n + col] / a[col * n + col];
            if f == 0.0 {
                continue;
            }
            for k in col..n {
                a[row * n + k] -= f * a[col * n + k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let s: f64 = (row + 1..n).map(|k| a[row * n + k] * x[k]).sum();
        x[row] = (b[row] - s) / a[row * n + row];
    }
    Some(x)
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Rodrigues: rotation vector to matrix.
pub fn rotvec_to_matrix(r: [f64; 3]) -> [[f64; 3]; 3] {
    let theta = norm(r);
    if theta < 1e-12 {
        return Pose::IDENTITY.rotation;
    }
    let [kx, ky, kz] = [r[0] / theta, r[1] / theta, r[2] / theta];
    let (s, c) = theta.sin_cos();
    let v = 1.0 - c;
    [
        [c + kx * kx * v, kx * ky * v - kz * s, kx * kz * v + ky * s],
        [ky * kx * v + kz * s, c + ky * ky * v, ky * kz * v - kx * s],
        [kz * kx * v - ky * s, kz * ky * v + kx * s, c + kz * kz * v],
    ]
}

/// Rodrigues: rotation matrix to vector, robust near 0 and π.
pub fn matrix_to_rotvec(m: [[f64; 3]; 3]) -> [f64; 3] {
    let trace = m[0][0] + m[1][1] + m[2][2];
    let cos = ((trace - 1.0) / 2.0).clamp(-1.0, 1.0);
    let theta = cos.acos();
    if theta < 1e-12 {
        return [0.0; 3];
    }
    let axis_from_skew = [m[2][1] - m[1][2], m[0][2] - m[2][0], m[1][0] - m[0][1]];
    if theta < std::f64::consts::PI - 1e-6 {
        let s = 2.0 * theta.sin();
        return [
            axis_from_skew[0] / s * theta,
            axis_from_skew[1] / s * theta,
            axis_from_skew[2] / s * theta,
        ];
    }
    // Near π the skew part vanishes; take the axis from the symmetric part.
    let mut axis = [
        ((m[0][0] + 1.0) / 2.0).max(0.0).sqrt(),
        ((m[1][1] + 1.0) / 2.0).max(0.0).sqrt(),
        ((m[2][2] + 1.0) / 2.0).max(0.0).sqrt(),
    ];
    // Fix the signs from the off-diagonal terms, relative to the largest component.
    let i = (0..3)
        .max_by(|&a, &b| axis[a].partial_cmp(&axis[b]).unwrap())
        .unwrap();
    for j in 0..3 {
        if j != i && m[i][j] < 0.0 {
            axis[j] = -axis[j];
        }
    }
    // Resolve the overall sign with the (small) skew part when it is informative.
    if (0..3).map(|k| axis[k] * axis_from_skew[k]).sum::<f64>() < 0.0 {
        axis.iter_mut().for_each(|v| *v = -*v);
    }
    let n = norm(axis);
    [
        axis[0] / n * theta,
        axis[1] / n * theta,
        axis[2] / n * theta,
    ]
}

fn rotvec_to_quat(r: [f64; 3]) -> [f64; 4] {
    let theta = norm(r);
    if theta < 1e-12 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    let s = (theta / 2.0).sin() / theta;
    [(theta / 2.0).cos(), r[0] * s, r[1] * s, r[2] * s]
}

fn quat_to_rotvec(q: [f64; 4]) -> [f64; 3] {
    let w = q[0].clamp(-1.0, 1.0);
    let theta = 2.0 * w.acos();
    let s = (1.0 - w * w).sqrt();
    if s < 1e-12 {
        return [0.0; 3];
    }
    [q[1] / s * theta, q[2] / s * theta, q[3] / s * theta]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rodrigues_round_trips_including_near_pi() {
        for r in [
            [0.0, 0.0, 0.0],
            [0.1, -0.2, 0.3],
            [1.0, 2.0, -0.5],
            [0.0, std::f64::consts::PI - 1e-9, 0.0],
            [2.2, 2.2, 0.1],
        ] {
            let back = matrix_to_rotvec(rotvec_to_matrix(r));
            let m1 = rotvec_to_matrix(r);
            let m2 = rotvec_to_matrix(back);
            for i in 0..3 {
                for j in 0..3 {
                    // acos loses precision near ±1, so the angle near π is only good to ~1e-8.
                    assert!((m1[i][j] - m2[i][j]).abs() < 1e-7, "{r:?} -> {back:?}");
                }
            }
        }
    }

    #[test]
    fn compose_and_inverse_are_consistent() {
        let a = Pose::from_rotvec([0.2, -0.1, 0.4], [1.0, 2.0, 3.0]);
        let b = Pose::from_rotvec([-0.3, 0.5, 0.1], [-0.5, 0.25, 1.5]);
        let p = [0.3, -0.7, 2.0];
        let via = a.apply(b.apply(p));
        let composed = a.compose(&b).apply(p);
        for i in 0..3 {
            assert!((via[i] - composed[i]).abs() < 1e-12);
        }
        let identity = a.compose(&a.inverse());
        assert!(identity.angle_to_deg(&Pose::IDENTITY) < 1e-9);
        assert!(identity.distance_to(&Pose::IDENTITY) < 1e-12);
    }

    #[test]
    fn average_of_identical_poses_is_the_pose_and_of_two_is_between() {
        let p = Pose::from_rotvec([0.3, 0.2, -0.1], [1.0, 0.0, 0.5]);
        let avg = average_pose(&[p, p, p]);
        assert!(avg.angle_to_deg(&p) < 1e-9 && avg.distance_to(&p) < 1e-12);
        let a = Pose::from_rotvec([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let b = Pose::from_rotvec([0.0, 0.0, 0.2], [0.0, 2.0, 0.0]);
        let mid = average_pose(&[a, b]);
        assert!((mid.rotvec()[2] - 0.1).abs() < 1e-9, "{:?}", mid.rotvec());
        assert!((mid.translation[1] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn dense_solver_solves_a_small_system() {
        let a = vec![4.0, 1.0, 0.0, 1.0, 3.0, 1.0, 0.0, 1.0, 2.0];
        let b = [1.0, 2.0, 3.0];
        let x = solve_dense(a.clone(), b.to_vec(), 3).unwrap();
        for i in 0..3 {
            let ax: f64 = (0..3).map(|k| a[i * 3 + k] * x[k]).sum();
            assert!((ax - b[i]).abs() < 1e-12, "{x:?}");
        }
        assert!(solve_dense(vec![1.0, 2.0, 2.0, 4.0], vec![1.0, 2.0], 2).is_none());
    }

    #[test]
    fn calibration_file_round_trips_and_rejects_other_formats() {
        let calibration = Calibration {
            format: CALIBRATION_FORMAT,
            board: BoardInfo::current(),
            reference: "/dev/video0".into(),
            cameras: vec![CameraCalibration {
                device_id: "/dev/video0".into(),
                image_size: [1920, 1080],
                camera_matrix: [[1000.0, 0.0, 960.0], [0.0, 1000.0, 540.0], [0.0, 0.0, 1.0]],
                dist_coeffs: vec![0.0; 5],
                rotation: Pose::IDENTITY.rotation,
                translation: [0.0; 3],
                intrinsics_rms_px: 0.2,
                extrinsics_rms_px: 0.3,
            }],
        };
        let text = calibration.to_json();
        assert!(
            text.contains("\"format\": 1") && text.contains("DICT_5X5_100"),
            "{text}"
        );
        assert_eq!(Calibration::from_json(&text).unwrap(), calibration);
        assert!(Calibration::from_json(&text.replace("\"format\": 1", "\"format\": 2")).is_err());
        assert_eq!(calibration.board.squares, [7, 5]);
    }
}
