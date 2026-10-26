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
