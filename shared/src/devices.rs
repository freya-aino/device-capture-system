use std::cmp::Ordering;
use std::thread::JoinHandle;

use anyhow::{Error, Result};
use bincode::{Decode, Encode};

use crate::FramePacket;

pub trait DeviceConfig {}

pub trait Device {
    type Config: DeviceConfig;

    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn device_type(&self) -> &DeviceType;
    fn device_status(&self) -> &DeviceStatus;
    fn get_configs(&self) -> Result<Vec<Self::Config>, Error>;
    fn start(
        &mut self,
        config: Self::Config,
        callback: Box<dyn Fn(FramePacket) -> Result<(), Error> + Send + 'static>,
    ) -> Result<JoinHandle<Result<(), Error>>, Error>;
    fn stop(&mut self) -> Result<(), Error>;
}

pub enum DeviceCommand {
    Start,
    Stop,
    Pause,
    Resume,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum DeviceType {
    Camera,
    Microphone,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum DeviceStatus {
    Available,
    Running,
    Stopped,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct DeviceInformation {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub device_status: DeviceStatus,
}

#[derive(Debug, Clone)]
pub struct MicrophoneConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: u32,
}

#[derive(Clone)]
pub struct CameraConfig {
    pub width: u32,
    pub height: u32,
    pub fps: (u32, u32),
    pub fourcc: [char; 4],
}

impl std::fmt::Debug for CameraConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CameraConfig {{ {}x{} @ {}/{} s/frame ({:.1} fps) {} }}",
            self.width,
            self.height,
            self.fps.0,
            self.fps.1,
            self.frames_per_second(),
            self.fourcc_string()
        )
    }
}

impl DeviceConfig for MicrophoneConfig {}
impl DeviceConfig for CameraConfig {}

impl CameraConfig {
    pub fn frames_per_second(&self) -> f64 {
        if self.fps.0 == 0 {
            return 0.0;
        }
        self.fps.1 as f64 / self.fps.0 as f64
    }

    pub fn area(&self) -> f64 {
        self.width as f64 * self.height as f64
    }

    /// The FourCC as a 4-character string, e.g. `"YUYV"` or `"MJPG"`. This is the value a
    /// camera frame carries in `FramePacketInformation::pixel_format`.
    pub fn fourcc_string(&self) -> String {
        self.fourcc.iter().collect()
    }

    /// Whether the FourCC names an uncompressed pixel format the consumer can use without a decoder.
    pub fn is_uncompressed(&self) -> bool {
        !matches!(
            self.fourcc_string().as_str(),
            "MJPG" | "JPEG" | "H264" | "HEVC" | "H265" | "VP80" | "VP90" | "AV01"
        )
    }
}

impl std::str::FromStr for CameraSelectionPolicy {
    type Err = String;

    /// Accepts `best-fps`, `best-resolution` and `best-average` (case-insensitive, `_` also allowed).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().replace('_', "-").as_str() {
            "best-fps" => Ok(CameraSelectionPolicy::BestFps),
            "best-resolution" => Ok(CameraSelectionPolicy::BestResolution),
            "best-average" => Ok(CameraSelectionPolicy::BestAverage),
            other => Err(format!(
                "unknown camera selection policy {other:?}; expected best-fps, best-resolution or best-average"
            )),
        }
    }
}

/// Ranking rule used by `select_camera_config_by_policy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraSelectionPolicy {
    /// Highest frame rate; the larger resolution breaks ties.
    BestFps,
    /// Largest resolution; the higher frame rate breaks ties.
    BestResolution,
    /// Highest mean of the resolution and frame rate, each scaled to the best value in the list.
    BestAverage,
}

/// Picks one advertised config according to `policy`. When two configs rank equally,
/// the one listed first wins.
pub fn select_camera_config_by_policy(
    configs: &[CameraConfig],
    policy: CameraSelectionPolicy,
) -> Option<CameraConfig> {
    let max_area = configs.iter().map(CameraConfig::area).fold(0.0, f64::max);
    let max_fps = configs
        .iter()
        .map(CameraConfig::frames_per_second)
        .fold(0.0, f64::max);
    let scale = |value: f64, max: f64| if max > 0.0 { value / max } else { 0.0 };

    let rank = |config: &CameraConfig| {
        let area = config.area();
        let fps = config.frames_per_second();
        match policy {
            CameraSelectionPolicy::BestFps => [fps, area, 0.0],
            CameraSelectionPolicy::BestResolution => [area, fps, 0.0],
            CameraSelectionPolicy::BestAverage => {
                let score = (scale(area, max_area) + scale(fps, max_fps)) / 2.0;
                [score, fps, area]
            }
        }
    };

    configs
        .iter()
        .reduce(|best, candidate| {
            if compare_ranks(&rank(candidate), &rank(best)) == Ordering::Greater {
                candidate
            } else {
                best
            }
        })
        .cloned()
}

fn compare_ranks(a: &[f64; 3], b: &[f64; 3]) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.total_cmp(y))
        .find(|ordering| ordering.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// Picks the advertised config closest to the requested resolution, then closest to the
/// requested frames per second. Exact resolution matches win over nearer frame rates at other sizes.
/// When two configs tie on both, an uncompressed FourCC (for example `YUYV`) wins over a
/// compressed one (for example `MJPG`); after that the one listed first wins.
pub fn select_camera_config(
    configs: &[CameraConfig],
    width: u32,
    height: u32,
    fps: f64,
) -> Option<CameraConfig> {
    let target_area = width as i64 * height as i64;
    let distance = |c: &CameraConfig| {
        let area = (c.width as i64 * c.height as i64 - target_area).abs();
        let rate = (c.frames_per_second() - fps).abs();
        // `false < true`, so uncompressed formats sort first.
        let compressed = !c.is_uncompressed();
        (area, rate, compressed)
    };

    configs
        .iter()
        .min_by(|a, b| {
            let (a_area, a_rate, a_compressed) = distance(a);
            let (b_area, b_rate, b_compressed) = distance(b);
            a_area
                .cmp(&b_area)
                .then(a_rate.total_cmp(&b_rate))
                .then(a_compressed.cmp(&b_compressed))
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_information_equality_depends_on_all_fields() {
        let base = DeviceInformation {
            id: "id".to_string(),
            name: "name".to_string(),
            device_type: DeviceType::Camera,
            device_status: DeviceStatus::Available,
        };

        let same = base.clone();
        assert_eq!(base, same);

        let mut different_status = base.clone();
        different_status.device_status = DeviceStatus::Running;
        assert_ne!(base, different_status);

        let mut different_type = base.clone();
        different_type.device_type = DeviceType::Microphone;
        assert_ne!(base, different_type);
    }

    fn config(width: u32, height: u32, fps: (u32, u32), fourcc: &str) -> CameraConfig {
        let mut chars = fourcc.chars();
        CameraConfig {
            width,
            height,
            fps,
            fourcc: std::array::from_fn(|_| chars.next().unwrap()),
        }
    }

    fn camera_config(width: u32, height: u32, fps: (u32, u32)) -> CameraConfig {
        CameraConfig {
            width,
            height,
            fps,
            fourcc: ['Y', 'U', 'Y', 'V'],
        }
    }

    #[test]
    fn select_camera_config_matches_resolution_and_fps() {
        let configs = vec![
            camera_config(1920, 1080, (1, 5)),
            camera_config(1920, 1080, (1, 30)),
            camera_config(1920, 1080, (1, 60)),
            camera_config(1280, 720, (1, 30)),
        ];

        let picked = select_camera_config(&configs, 1920, 1080, 30.0).unwrap();
        assert_eq!(
            (picked.width, picked.height, picked.fps),
            (1920, 1080, (1, 30))
        );

        let picked = select_camera_config(&configs, 1920, 1080, 60.0).unwrap();
        assert_eq!(picked.fps, (1, 60));

        let picked = select_camera_config(&configs, 1920, 1080, 4.0).unwrap();
        assert_eq!(picked.fps, (1, 5));
    }

    #[test]
    fn select_camera_config_falls_back_to_nearest_resolution() {
        let configs = vec![
            camera_config(1920, 1080, (1, 30)),
            camera_config(1280, 720, (1, 30)),
        ];

        let picked = select_camera_config(&configs, 1600, 900, 30.0).unwrap();
        assert_eq!((picked.width, picked.height), (1280, 720));
    }

    #[test]
    fn select_camera_config_prefers_uncompressed_on_exact_tie() {
        let configs = vec![
            config(640, 480, (1, 30), "MJPG"),
            config(640, 480, (1, 30), "YUYV"),
        ];
        let picked = select_camera_config(&configs, 640, 480, 30.0).unwrap();
        assert_eq!(picked.fourcc_string(), "YUYV");

        // Resolution still wins over format: 1080p is only MJPG here, and it is what was asked for.
        let framework = vec![
            config(1920, 1080, (1, 60), "MJPG"),
            config(1920, 1080, (1, 30), "MJPG"),
            config(640, 480, (1, 30), "YUYV"),
        ];
        let picked = select_camera_config(&framework, 1920, 1080, 30.0).unwrap();
        assert_eq!(
            (picked.width, picked.height, picked.fps),
            (1920, 1080, (1, 30))
        );
        assert_eq!(picked.fourcc_string(), "MJPG");
    }

    #[test]
    fn camera_selection_policy_parses_from_str() {
        assert_eq!("best-fps".parse(), Ok(CameraSelectionPolicy::BestFps));
        assert_eq!(
            "BEST_RESOLUTION".parse(),
            Ok(CameraSelectionPolicy::BestResolution)
        );
        assert_eq!(
            "best-average".parse(),
            Ok(CameraSelectionPolicy::BestAverage)
        );
        assert!("nearest".parse::<CameraSelectionPolicy>().is_err());
    }

    #[test]
    fn camera_config_fourcc_string_and_compression() {
        let yuyv = config(640, 480, (1, 30), "YUYV");
        assert_eq!(yuyv.fourcc_string(), "YUYV");
        assert!(yuyv.is_uncompressed());
        assert!(!config(640, 480, (1, 30), "MJPG").is_uncompressed());
        assert_eq!(
            format!("{:?}", yuyv),
            "CameraConfig { 640x480 @ 1/30 s/frame (30.0 fps) YUYV }"
        );
    }

    #[test]
    fn select_camera_config_returns_none_for_empty_list() {
        assert!(select_camera_config(&[], 1920, 1080, 30.0).is_none());
    }

    #[test]
    fn frames_per_second_handles_zero_numerator() {
        assert_eq!(camera_config(640, 480, (1, 30)).frames_per_second(), 30.0);
        assert_eq!(camera_config(640, 480, (0, 30)).frames_per_second(), 0.0);
    }

    #[test]
    fn policy_best_fps_prefers_highest_frame_rate() {
        let configs = vec![
            camera_config(1920, 1080, (1, 5)),
            camera_config(1280, 720, (1, 60)),
            camera_config(640, 480, (1, 30)),
        ];

        let picked =
            select_camera_config_by_policy(&configs, CameraSelectionPolicy::BestFps).unwrap();
        assert_eq!((picked.width, picked.height), (1280, 720));
    }

    #[test]
    fn policy_best_fps_uses_resolution_to_break_ties() {
        let configs = vec![
            camera_config(640, 480, (1, 30)),
            camera_config(1920, 1080, (1, 30)),
        ];

        let picked =
            select_camera_config_by_policy(&configs, CameraSelectionPolicy::BestFps).unwrap();
        assert_eq!((picked.width, picked.height), (1920, 1080));
    }

    #[test]
    fn policy_best_resolution_prefers_largest_area() {
        let configs = vec![
            camera_config(1920, 1080, (1, 5)),
            camera_config(1280, 720, (1, 60)),
            camera_config(640, 480, (1, 30)),
        ];

        let picked =
            select_camera_config_by_policy(&configs, CameraSelectionPolicy::BestResolution)
                .unwrap();
        assert_eq!((picked.width, picked.height), (1920, 1080));
        assert_eq!(picked.fps, (1, 5));
    }

    #[test]
    fn policy_best_resolution_uses_fps_to_break_ties() {
        let configs = vec![
            camera_config(1920, 1080, (1, 15)),
            camera_config(1920, 1080, (1, 30)),
        ];

        let picked =
            select_camera_config_by_policy(&configs, CameraSelectionPolicy::BestResolution)
                .unwrap();
        assert_eq!(picked.fps, (1, 30));
    }

    #[test]
    fn policy_best_average_balances_resolution_and_fps() {
        let configs = vec![
            camera_config(1920, 1080, (1, 10)),
            camera_config(1280, 720, (1, 45)),
            camera_config(640, 480, (1, 60)),
        ];

        let picked =
            select_camera_config_by_policy(&configs, CameraSelectionPolicy::BestAverage).unwrap();
        assert_eq!((picked.width, picked.height), (1280, 720));
        assert_eq!(picked.fps, (1, 45));
    }

    #[test]
    fn policy_best_average_handles_zero_frame_rate() {
        let configs = vec![camera_config(640, 480, (0, 30))];

        let picked =
            select_camera_config_by_policy(&configs, CameraSelectionPolicy::BestAverage).unwrap();
        assert_eq!((picked.width, picked.height), (640, 480));
    }

    #[test]
    fn policy_selection_returns_none_for_empty_list() {
        for policy in [
            CameraSelectionPolicy::BestFps,
            CameraSelectionPolicy::BestResolution,
            CameraSelectionPolicy::BestAverage,
        ] {
            assert!(select_camera_config_by_policy(&[], policy).is_none());
        }
    }

    #[test]
    fn device_information_encode_decode_roundtrip() {
        let info = DeviceInformation {
            id: "id".to_string(),
            name: "name".to_string(),
            device_type: DeviceType::Microphone,
            device_status: DeviceStatus::Stopped,
        };

        let encoded = bincode::encode_to_vec(&info, bincode::config::standard()).unwrap();
        let (decoded, _): (DeviceInformation, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        assert_eq!(info, decoded);
    }
}
