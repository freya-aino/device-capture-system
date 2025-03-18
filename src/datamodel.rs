use std::ops::Deref;
use std::time::SystemTime;

use anyhow::Error;
use either::{Either, IntoEither};

use cpal::traits::{DeviceTrait, HostTrait};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType};
use nokhwa::{native_api_backend, nokhwa_check, Camera};
use nokhwa::query;


// ---------------- ENUMS ----------------

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    Camera,
    Microphone,
}

#[derive(Debug, Clone)]
pub enum DeviceStatus {
    Available,
    Capturing,
    Paused,
    Error(String),
    Disabled,
}

#[derive(Debug, Clone)]
pub enum DeviceConfig {
    Camera(CameraConfig),
    Microphone(MicrophoneConfig),
}

// ---------------- STRUCTS ----------------

#[derive(Debug, Clone)]
pub struct DeviceId(pub u32);

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    device_type: DeviceType,
    state: DeviceStatus
}

#[derive(Debug, Clone)]
pub struct CameraConfig {
    resolution: (u32, u32),
    fps: u32,
    pixel_format: FrameFormat,
}

#[derive(Debug, Clone)]
pub struct MicrophoneConfig {
    sample_rate: u32,
    channels: u32,
    sample_size: u32,
}

#[derive(Debug, Clone)]
pub struct CaptureStats {
    frames_captured: u64,
    frames_droped: u64,
    bytes_captured: u64,
    current_fps: f32,
    current_bitrate: f32,
    current_latency: f32,
    uptime_seconds: u64,
    last_frame_time: SystemTime,
}

#[derive(Debug, Clone)]
pub struct Device {
    device_info: DeviceInfo,
    configs: Vec<DeviceConfig>,
    selected_config: Option<DeviceConfig>,
}

#[derive(Debug, Clone)]
pub struct CaptureProcess {
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

impl DeviceInfo {
    pub fn new(id: DeviceId, name: String, device_type: DeviceType, state: DeviceStatus) -> Self {
        DeviceInfo {
            id: id,
            name: name,
            device_type: device_type,
            state: state,
        }
    }

    fn get_all_camera_configs(id: u32) -> Result<Vec<CameraConfig>, Error> {
        let mut out_configs = Vec::new();

        let cam = Camera::new(
            CameraIndex::Index(id),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
        );

        let formats = match cam.unwrap() {
            mut cam => cam.compatible_camera_formats().unwrap(),
            _ => Vec::new(),
        };

        for format in formats {
            out_configs.push(
                CameraConfig {
                    resolution: (format.resolution().width(), format.resolution().height()),
                    fps: format.frame_rate(),
                    pixel_format: format.format(),
                }.clone()
            );
        }

        Ok(out_configs)
    }

    fn get_all_microphone_configs(id: u32) -> Result<Vec<MicrophoneConfig>, Error> {
        let mut out_configs = Vec::new();
        
        let host = cpal::default_host();
        let device = host.input_devices().unwrap().nth(id as usize).unwrap();
        let supported_configs = device.supported_input_configs().unwrap();

        for conf in supported_configs {
            out_configs.push(
                MicrophoneConfig {
                    sample_rate: conf.max_sample_rate().0 as u32,
                    channels: conf.channels() as u32,
                    sample_size: conf.sample_format().sample_size() as u32,
                }
            );
        }
        Ok(out_configs)
    }
}

impl Device {
    pub fn new(device_info: DeviceInfo) -> Self {

        let configs = match device_info.device_type {
            DeviceType::Camera => {
                DeviceInfo::get_all_camera_configs(device_info.id.0 as u32)
                    .unwrap()
                    .into_iter()
                    .map(DeviceConfig::Camera)
                    .collect()
            },
            DeviceType::Microphone => {
                DeviceInfo::get_all_microphone_configs(device_info.id.0 as u32)
                    .unwrap()
                    .into_iter()
                    .map(DeviceConfig::Microphone)
                    .collect()
            },
        };

        println!("Device created : {:?}", device_info.name);
        
        Device {
            device_info: device_info,
            configs: configs,
            selected_config: None,
        }
    }
    
    pub fn get_all_cameras() -> Result<Vec<DeviceInfo>, Error> {
        let mut out_cameras = Vec::new();

        match nokhwa_check() {
            true => println!("Nokhwa is available"),
            false => panic!("Nokhwa is not available"),
        }

        let backend = native_api_backend()
            .expect("failed to get native api backend");

        let camera_infos = query(backend)
            .expect("failed to query cameras");
        
        for device in camera_infos {

            out_cameras.push(
                DeviceInfo::new(
                    DeviceId(device.index().as_index().unwrap() as u32),
                    device.human_name().to_string(),
                    DeviceType::Camera,
                    DeviceStatus::Available
                )
            )
        }
        Ok(out_cameras)
    }
}

    // fn get_all_microphones() -> Result<Vec<Device>, Error> {
    //     let mut out_microphones = Vec::new();
    //     let host = cpal::default_host();
    //     let devices = host.input_devices().unwrap();
    //     for (i, device) in devices.enumerate() {
    //         out_microphones.push(
    //             Device::new(
    //                 i as u16,
    //                 device.name().unwrap(),
    //                 None,
    //             )
    //         );
    //     }
    //     Ok(out_microphones)
    // }
// }


// impl CaptureProcess {
//     pub fn new(device_info: DeviceInfo, process_id: u32, port: u32, config: CaptureConfig) -> Self {
//         CaptureProcess {
//             device_info: device_info,
//             process_id: process_id,
//             port: port,
//             start_time: SystemTime::now(),
//             config: config,
//             stats: CaptureStats {
//                 frames_captured: 0,
//                 frames_droped: 0,
//                 bytes_captured: 0,
//                 current_fps: 0.0,
//                 current_bitrate: 0.0,
//                 current_latency: 0.0,
//                 uptime_seconds: 0,
//                 last_frame_time: SystemTime::now(),
//             },
//         }
//     }
// }
