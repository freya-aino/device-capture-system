use std::time::Duration;

use anyhow::Error;
use bincode::{Decode, Encode};

use crate::FramePacket;

pub trait DeviceConfig {}

pub trait Device {
    type Config: DeviceConfig;

    fn id(&self) -> u16;
    fn name(&self) -> &str;
    fn device_type(&self) -> &DeviceType;
    fn get_configs(&self) -> Result<Vec<Self::Config>, Error>;
    fn open(
        &mut self,
        config: Self::Config,
        callback: Box<dyn Fn(FramePacket) + Send + 'static>,
        timeout: Option<Duration>,
    ) -> Result<(), Error>;
    fn close(&mut self) -> Result<(), Error>;
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

#[derive(Encode, Decode)]
pub enum DeviceStatus {
    Available,
    Running,
    Paused,
    Stopped,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct DeviceInformation {
    pub id: u16,
    pub name: String,
    pub device_type: DeviceType,
}

#[derive(Debug, Clone)]
pub struct MicrophoneConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: u32,
}

#[derive(Debug, Clone)]
pub struct CameraConfig {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub fourcc: [u8; 4],
}

impl DeviceConfig for MicrophoneConfig {}
impl DeviceConfig for CameraConfig {}
