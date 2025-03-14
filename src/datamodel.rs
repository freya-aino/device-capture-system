use std::path::Path;

// datamodel
enum DeviceType {
    Camera(nokhwa::Camera),
    Microphone(cpal::Device),
}

enum DeviceConfig {
    Camera(CameraConfig),
    Microphone(MicrophoneConfig),
}

trait Device {
    fn get_name(&self) -> String;
    fn get_type(&self) -> DeviceType;
    fn get_configs(&self) -> Vec<DeviceConfig>;
}


struct CameraConfig {
    width: u16,
    height: u16,
    frame_rate: u16,
    pixel_format: FrameFormat,
}

struct MicrophoneConfig {
    channels: u8,
    sample_rate: u32,
    sample_size: u8,
}


pub struct CameraDevice {
    name: String,
    index: nokhwa::utils::CameraIndex,
    id: Option<String>,
    config: Vec<CameraConfig>,
}

struct MicrophoneDevice {
    name: String,
    microphone_device: String,
    microphone_config: Vec<MicrophoneConfig>,
}


impl CameraDevice {
    pub fn new(index: nokhwa::utils::CameraIndex, name: String, id: Option<String>, config: Option<Vec<CameraConfig>>) -> Self {
        let config = match config {
            Some(config) => config,
            None => Vec::new(),
        };
        CameraDevice {
            name: name,
            index: index,
            id: id,
            config: config,
        }
    }
}