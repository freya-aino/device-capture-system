use std::time::SystemTime;

// ---------------- ENUMS ----------------

enum DeviceType {
    Camera,
    Microphone,
}

enum DeviceStatus {
    Available,
    Capturing,
    Paused,
    Error(String),
    Disabled,
}

enum DeviceConfig {
    Camera(CameraConfig),
    Microphone(MicrophoneConfig),
}

enum PixelFormat {
    RGB,
}

// ---------------- STRUCTS ----------------

#[derive(Debug, Clone)]
pub struct DeviceId(pub u32);

struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    device_type: DeviceType,
    state: DeviceStatus,
}

struct CameraConfig {
    resolution: (u32, u32),
    fps: u32,
    pixel_format: PixelFormat,
}

struct MicrophoneConfig {
    sample_rate: u32,
    channels: u32,
    sample_size: u32,
}

struct CaptureStats {
    frames_captured: u64,
    frames_droped: u64,
    bytes_captured: u64,
    current_fps: f32,
    current_bitrate: f32,
    current_latency: f32,
    start_time: SystemTime,
    last_frame_time: SystemTime,
}

struct Device {
    device_info: DeviceInfo,
    configs: Vec<DeviceConfig>,
    selected_config: Option<DeviceConfig>,
}

struct CaptureProcess {
    device: Device,
    process_id: u32,
    port: u32,
    start_time: SystemTime,
    config: DeviceConfig,
    stats: CaptureStats,
}

// ---------------- TRAITS ----------------
// trait Process {
//     fn initialize(&self) -> Result<(), Error>;
//     fn start(&self) -> Result<(), Error>;
//     fn stop(&self) -> Result<(), Error>;
//     fn pause(&self) -> Result<(), Error>;
//     fn resume(&self) -> Result<(), Error>;
//     fn restart(&self) -> Result<(), Error>;
//     fn get_stats(&self) -> CaptureStats;
// }

// ---------------- IMPLS ----------------
