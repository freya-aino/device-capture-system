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

// impl std::fmt::Debug for CameraConfig {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let parsed = String::from_utf8(self.fourcc.to_vec()).unwrap();

//         write!(
//             f,
//             "CameraConfig {{ width: {}, height: {}, fps: {}/{}, fourcc: {} }}",
//             self.width, self.height, self.fps.0, self.fps.1, parsed
//         )
//     }
// }

impl DeviceConfig for MicrophoneConfig {}
impl DeviceConfig for CameraConfig {}

impl CameraConfig {
    pub fn frames_per_second(&self) -> f64 {
        if self.fps.0 == 0 {
            return 0.0;
        }
        self.fps.1 as f64 / self.fps.0 as f64
    }
}

/// Picks the advertised config closest to the requested resolution, then closest to the
/// requested frames per second. Exact resolution matches win over nearer frame rates at other sizes.
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
        (area, rate)
    };

    configs
        .iter()
        .min_by(|a, b| {
            let (a_area, a_rate) = distance(a);
            let (b_area, b_rate) = distance(b);
            a_area.cmp(&b_area).then(a_rate.total_cmp(&b_rate))
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
        assert_eq!((picked.width, picked.height, picked.fps), (1920, 1080, (1, 30)));

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
    fn select_camera_config_returns_none_for_empty_list() {
        assert!(select_camera_config(&[], 1920, 1080, 30.0).is_none());
    }

    #[test]
    fn frames_per_second_handles_zero_numerator() {
        assert_eq!(camera_config(640, 480, (1, 30)).frames_per_second(), 30.0);
        assert_eq!(camera_config(640, 480, (0, 30)).frames_per_second(), 0.0);
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
