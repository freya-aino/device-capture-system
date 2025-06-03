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
        callback: Box<dyn Fn(FramePacket) + Send + 'static>,
    ) -> Result<JoinHandle<()>, Error>;
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
    pub fourcc: [u8; 4],
}

impl std::fmt::Debug for CameraConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parsed = String::from_utf8(self.fourcc.to_vec()).unwrap();

        write!(
            f,
            "CameraConfig {{ width: {}, height: {}, fps: {}/{}, fourcc: {} }}",
            self.width, self.height, self.fps.0, self.fps.1, parsed
        )
    }
}

impl DeviceConfig for MicrophoneConfig {}
impl DeviceConfig for CameraConfig {}
