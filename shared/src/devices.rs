use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum DeviceType {
    Camera,
    Microphone,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct DeviceInformation {
    pub id: u16,
    pub name: String,
    pub device_type: DeviceType,
}

#[derive(Debug)]
pub struct CameraConfig {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub fourcc: String,
}

#[derive(Debug)]
pub struct MicrophoneConfig {
    pub sample_rate: u32,
    pub channels: u8,
    pub sample_size: u16,
}
