use anyhow::{Error, Result};
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use std::time::Instant;
use nokhwa::utils::FrameFormat as PixelFormat;
use clap::ValueEnum;
use std::process::{Child, Command};
use std::sync::mpsc::{channel, Sender, Receiver};


#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum DeviceType {
    Camera,
    Microphone,
}

#[derive(Debug)]
pub enum DeviceStatus {
    Initialized,
    Started,
    Paused,
    Stopped,
    Terminated,
}

#[derive(Debug)]
enum ProcessControl {
    Start,
    Stop,
    Pause,
    Resume,
    Terminate
}

// #[derive(Debug)]
// pub enum DeviceData {
//     CameraData(CameraData),
//     MicrophoneData(MicrophoneData),
// }

// ---

trait DeviceConfig {}

pub trait Device {
    type Config: DeviceConfig;

    fn get_all_available_devices() -> Result<Vec<DeviceInformation>, Error>;

    fn new(id: u8, name: String) -> Self;
    fn get_info(&self) -> DeviceInformation;

    fn get_available_configs(&self) -> Result<Vec<Self::Config>, Error>;
    
    // fn start(&self) -> Result<(), Error>;
    // fn stop(&self) -> Result<(), Error>;
    // fn pause(&self) -> Result<(), Error>;
    // fn resume(&self) -> Result<(), Error>;
}

// ---

#[derive(Debug, Clone)]
pub struct DeviceInformation {
    pub id: u8,
    pub name: String,
    pub device_type: DeviceType,
}

#[derive(Debug)]
pub struct CameraConfig {
    width: u32,
    height: u32,
    fps: u32,
    pixel_format: PixelFormat,
}

#[derive(Debug)]
pub struct MicrophoneConfig {
    sample_rate: u32,
    channels: u32,
    sample_size: u32,
}

#[derive(Debug)]
pub struct CameraDevice(DeviceInformation);

#[derive(Debug)]
pub struct MicrophoneDevice(DeviceInformation);

#[derive(Debug)]
pub struct FrameData {
    frame: Vec<u8>,
    device_type: DeviceType,
    timestamp: Instant,
}

// ---

impl DeviceConfig for CameraConfig {}
impl DeviceConfig for MicrophoneConfig {}

impl Device for CameraDevice {
    type Config = CameraConfig;

    fn new(id: u8, name: String) -> Self {
        CameraDevice(DeviceInformation {
            id: id,
            name: name,
            device_type: DeviceType::Camera,
        })
    }
    fn get_info(&self) -> DeviceInformation { self.0.clone() }

    fn get_available_configs(&self) -> Result<Vec<Self::Config>, Error> {
        todo!("Implement get all configs for camera");
    }
    fn get_all_available_devices() -> Result<Vec<DeviceInformation>, Error> {
        let nokhwa_backend = nokhwa::native_api_backend().unwrap();
        let devices = nokhwa::query(nokhwa_backend)
                .unwrap()
                .into_iter()
                .map(|device| {
                    DeviceInformation {
                        id: device.index().as_index().unwrap() as u8,
                        name: device.human_name().to_string(),
                        device_type: DeviceType::Camera,
                    }
                })
                .collect::<Vec<DeviceInformation>>();
        Ok(devices)
    }

    // fn start(&self) -> Result<(), Error> {
    //     // Start capturing from the camera device
    //     todo!("Implement start for camera device");
    // }

    // fn stop(&self) -> Result<(), Error> {
    //     todo!("Implement stop for camera device");
    // }
}

impl Device for MicrophoneDevice {
    type Config = MicrophoneConfig;

    fn new(id: u8, name: String) -> Self {
        MicrophoneDevice(DeviceInformation { 
            id: id,
            name: name,
            device_type: DeviceType::Microphone,
        })
    }
    fn get_info(&self) -> DeviceInformation { self.0.clone() }
    
    fn get_available_configs(&self) -> Result<Vec<Self::Config>, Error> {
        let host = cpal::default_host();
        let supported_configs = host.input_devices()?
            .nth(self.0.id as usize)
            .unwrap()
            .supported_input_configs()?;

        let mut out_configs = Vec::new();
        for conf in supported_configs {
            out_configs.push(MicrophoneConfig {
                sample_rate: conf.min_sample_rate().0,
                channels: conf.channels() as u32,
                sample_size: conf.sample_format().sample_size() as u32,
            });
        }
        Ok(out_configs)
    }

    fn get_all_available_devices() -> Result<Vec<DeviceInformation>, Error> {
        let host = cpal::default_host();
        let devices = host.input_devices()?;
        let mut out_devices = Vec::new();
        for (i, device) in devices.enumerate() {
            out_devices.push(DeviceInformation {
                id: i as u8,
                name: device.name().unwrap(),
                device_type: DeviceType::Microphone,
            });
        }
        Ok(out_devices)
    }

    // fn start(&self) -> Result<(), Error> {
    //     // Start capturing from the microphone device
    //     todo!("Implement start for microphone device");
    // }

    // fn stop(&self) -> Result<(), Error> {
    //     // Stop capturing from the microphone device
    //     todo!("Implement stop for microphone device");
    // }
}

// ---

#[derive(Debug)]
pub struct DeviceManager {
    pub system_id: u8,
    pub device_info: DeviceInformation,
    pub config_id: Option<u8>,
    pub status: DeviceStatus,
}

impl DeviceManager {
    pub fn new(device_info: DeviceInformation, system_id: u8, config_id: Option<u8>) -> Self {
        DeviceManager {
            device_info: device_info,
            config_id: config_id,
            system_id: system_id,
            status: DeviceStatus::Initialized,
        }
    }

    pub fn get_all_available_devices_managed() -> Result<Vec<Self>, Error> {
        let mut devices = Vec::new();
        devices.extend(CameraDevice::get_all_available_devices()?);
        devices.extend(MicrophoneDevice::get_all_available_devices()?);

        let mut managed_devices = devices
            .iter()
            .enumerate()
            .map(|(i, device)| {
                DeviceManager::new(device.clone(), i as u8, None)
            })
            .collect::<Vec<DeviceManager>>();

        Ok(managed_devices)
    }
}


// impl Device {
//     pub fn new(id: DeviceId, name: String, device_type: DeviceType) -> Self {
//         Device {
//             id: id,
//             name: name,
//             device_type: device_type,
//             state: DeviceStatus::Initialized,
//         }
//     }

//     // fn get_all_camera_configs(id: u32) -> Result<Vec<CameraConfig>, Error> {
//     //     let mut out_configs = Vec::new();

//     //     let cam = Camera::new(
//     //         CameraIndex::Index(id),
//     //         RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
//     //     );

//     //     let formats = match cam.unwrap() {
//     //         mut cam => cam.compatible_camera_formats().unwrap(),
//     //         _ => Vec::new(),
//     //     };

//     //     for format in formats {
//     //         out_configs.push(
//     //             CameraConfig {
//     //                 resolution: (format.resolution().width(), format.resolution().height()),
//     //                 fps: format.frame_rate(),
//     //                 pixel_format: format.format(),
//     //             }
//     //             .clone(),
//     //         );
//     //     }

//     //     Ok(out_configs)
//     // }

//     // fn get_all_microphone_configs(id: u32) -> Result<Vec<MicrophoneConfig>, Error> {
//     //     let mut out_configs = Vec::new();

//     //     let host = cpal::default_host();
//     //     let device = host.input_devices().unwrap().nth(id as usize).unwrap();
//     //     let supported_configs = device.supported_input_configs().unwrap();

//     //     for conf in supported_configs {
//     //         out_configs.push(MicrophoneConfig {
//     //             sample_rate: conf.max_sample_rate().0 as u32,
//     //             channels: conf.channels() as u32,
//     //             sample_size: conf.sample_format().sample_size() as u32,
//     //         });
//     //     }
//     //     Ok(out_configs)
//     // }
// }

// pub fn get_configs(&self) -> Result<(), Error> {
//     let conf = match self.device_info.device_type {
//         DeviceType::Camera => DeviceInfo::get_all_camera_configs(self.device_info.id.0 as u32)
//             .unwrap()
//             .into_iter()
//             .map(DeviceConfig::Camera)
//             .collect(),
//         DeviceType::Microphone => {
//             DeviceInfo::get_all_microphone_configs(self.device_info.id.0 as u32)
//                 .unwrap()
//                 .into_iter()
//                 .map(DeviceConfig::Microphone)
//                 .collect()
//         }
//     };
//     return Ok(());
// }

//     pub fn get_all_cameras() -> Result<Vec<DeviceInfo>, Error> {
//         let mut out_cameras = Vec::new();

//         match nokhwa_check() {
//             true => println!("Nokhwa is available"),
//             false => panic!("Nokhwa is not available"),
//         }

//         let backend = native_api_backend().expect("failed to get native api backend");

//         let camera_infos = query(backend).expect("failed to query cameras");

//         for device in camera_infos {
//             out_cameras.push(DeviceInfo::new(
//                 DeviceId(device.index().as_index().unwrap() as u32),
//                 device.human_name().to_string(),
//                 DeviceType::Camera,
//                 DeviceStatus::Available,
//             ))
//         }
//         Ok(out_cameras)
//     }
// }

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