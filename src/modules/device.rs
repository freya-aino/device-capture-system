use anyhow::Ok;
use anyhow::{Error, Result};
use bincode::{Decode, Encode};
use clap::ValueEnum;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use nokhwa::Camera;
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::FrameFormat as PixelFormat;
use nokhwa::utils::RequestedFormat;
use nokhwa::utils::RequestedFormatType;
use nokhwa::utils::{ApiBackend, CameraIndex};
use std::sync::Once;

// --- Config ---

// trait DeviceConfig {}

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
    channels: u8,
    sample_size: u16,
}

#[derive(Debug)]
pub enum DeviceConfig {
    Camera(CameraConfig),
    Microphone(MicrophoneConfig),
}

pub enum DevicePrimitive {
    Camera(Camera),
    Microphone(cpal::Device),
}

// impl DeviceConfig for CameraConfig {}
// impl DeviceConfig for MicrophoneConfig {}

// --- device ---

#[derive(Debug, Clone, ValueEnum, PartialEq, Encode, Decode)]
pub enum DeviceType {
    Camera,
    Microphone,
}

#[derive(Debug, PartialEq)]
pub enum DeviceStatus {
    Created,
    Initialized,
    Started,
    Paused,
    Stopped,
    Terminated,
}
// pub trait Device {
//     // fn get_all_available_devices() -> Result<Vec<DeviceInformation>, Error>;
//     fn get_all_available_configs(&mut self) -> Result<Vec<DeviceConfig>, Error>;

//     fn new(device_primitive: DevicePrimitive) -> Self;

//     fn start(&mut self) -> Result<(), Error>;
// }

#[derive(Debug, Clone, Encode, Decode)]
pub struct DeviceInformation {
    pub id: u8,
    pub name: String,
    pub device_type: DeviceType,
}

pub struct CameraDevice {
    device_information: DeviceInformation,
    camera: Camera,
    status: DeviceStatus,
}

pub struct MicrophoneDevice {
    device_information: DeviceInformation,
    microphone: cpal::Device,
    status: DeviceStatus,
}

impl CameraDevice {
    pub fn new(id: u8, name: String) -> CameraDevice {
        let camera = Camera::new(
            CameraIndex::Index(id as u32),
            RequestedFormat::new::<RgbAFormat>(RequestedFormatType::None),
        )
        .unwrap();

        CameraDevice {
            device_information: DeviceInformation {
                id: id,
                name: name,
                device_type: DeviceType::Camera,
            },
            camera: camera,
            status: DeviceStatus::Initialized,
        }
    }
}

impl MicrophoneDevice {
    pub fn new(host: &cpal::Host, id: u8, name: String) -> MicrophoneDevice {
        let mic = host
            .input_devices()
            .unwrap()
            .nth(id as usize)
            .ok_or_else(|| Error::msg("Device not found"))
            .unwrap();
        MicrophoneDevice {
            device_information: DeviceInformation {
                id: id,
                name: name,
                device_type: DeviceType::Microphone,
            },
            microphone: mic,
            status: DeviceStatus::Initialized,
        }
    }
}

// impl Device for CameraDevice {
//     fn get_all_available_configs(&mut self) -> Result<Vec<DeviceConfig>, Error> {
//         let formats = self.camera.compatible_camera_formats()?;

//         let mut cam_formats = formats
//             .into_iter()
//             .map(|conf| CameraConfig {
//                 width: conf.width(),
//                 height: conf.height(),
//                 fps: conf.frame_rate(),
//                 pixel_format: conf.format(),
//             })
//             .collect::<Vec<CameraConfig>>();

//         cam_formats.sort_by_key(|conf| (conf.width * conf.height, conf.fps));

//         Ok(cam_formats
//             .into_iter()
//             .map(|conf| DeviceConfig::Camera(conf))
//             .collect::<Vec<DeviceConfig>>())
//     }

//     // let config = CameraFormat::new_from(
//     //     conf.width,
//     //     conf.height,
//     //     conf.pixel_format,
//     //     conf.fps,
//     // );

//     // fn instantiate_device(&mut self) -> Result<(), Error> {
//     //     self.camera = Some(cam;
//     //     self.status = DeviceStatus::Initialized;
//     //     Ok(())
//     // }

//     fn start(&mut self) -> Result<(), Error> {
//         // assert!(self.device.is_some(), "Trying to start without Device initialized");
//         Ok(())
//     }
// }

// impl Device for MicrophoneDevice {
//     fn get_all_available_configs(&mut self) -> Result<Vec<DeviceConfig>, Error> {
//         let mic = self.device.as_ref().unwrap();
//         let supported_configs = mic.supported_input_configs()?;

//         let mut out_configs = Vec::new();
//         for conf in supported_configs {
//             out_configs.push(MicrophoneConfig {
//                 sample_rate: conf.max_sample_rate().0,
//                 channels: conf.channels() as u8,
//                 sample_size: conf.sample_format().sample_size() as u16,
//             });
//         }

//         out_configs.sort_by_key(|conf| (conf.sample_rate, conf.sample_size, conf.channels));

//         Ok(out_configs
//             .into_iter()
//             .map(|conf| DeviceConfig::Microphone(conf))
//             .collect::<Vec<DeviceConfig>>())
//     }
// }

// --- device manager ---

pub enum Device {
    Camera(CameraDevice),
    Microphone(MicrophoneDevice),
}

impl Device {
    pub fn get_device_information(&self) -> DeviceInformation {
        match self {
            Device::Camera(c) => c.device_information.clone(),
            Device::Microphone(m) => m.device_information.clone(),
        }
    }
}

// pub fn get_all_available_configs(&mut self) -> Result<Vec<DeviceConfig>, Error> {
//     match self {
//         ManagedDevice::Camera(c) => Ok(c.get_all_available_configs()?),
//         ManagedDevice::Microphone(m) => Ok(m.get_all_available_configs()?),
//     }
// }

// fn get_all_available_configs(&self) -> Result<Vec<DeviceConfig>, Error> {
//     match self {
//         ManagedDevice::Camera(_) => self.get_all_available_configs(),
//         ManagedDevice::Microphone(_) => self.get_all_available_configs(),
//     }
// }
// }

pub struct DeviceManager {
    pub system_id: u8,
    pub device: Device,
}

impl DeviceManager {
    pub fn get_all_available_devices(
        nokhwa_backend: &ApiBackend,
        cpal_host: &cpal::Host,
    ) -> Result<Vec<Self>, Error> {
        let cameras = DeviceManager::get_all_available_cameras(nokhwa_backend)?;
        let microphones = DeviceManager::get_all_available_microphones(cpal_host)?;

        let mut all_devices = Vec::new();
        let mut id_counter = 0;
        for cam in cameras {
            all_devices.push(DeviceManager {
                system_id: id_counter,
                device: Device::Camera(cam),
            });
            id_counter += 1;
        }
        for mic in microphones {
            all_devices.push(DeviceManager {
                system_id: id_counter,
                device: Device::Microphone(mic),
            });
            id_counter += 1;
        }

        Ok(all_devices)
    }

    pub fn get_all_available_microphones(
        host: &cpal::Host,
    ) -> Result<Vec<MicrophoneDevice>, Error> {
        let devices = host
            .input_devices()?
            .enumerate()
            .map(|(i, device)| MicrophoneDevice::new(host, i as u8, device.name().unwrap()))
            .collect::<Vec<MicrophoneDevice>>();
        Ok(devices)
    }

    pub fn get_all_available_cameras(
        nokhwa_backend: &ApiBackend,
    ) -> Result<Vec<CameraDevice>, Error> {
        let devices = nokhwa::query(*nokhwa_backend)
            .unwrap()
            .into_iter()
            .map(|device| {
                CameraDevice::new(
                    device.index().as_index().unwrap() as u8,
                    device.human_name().to_string(),
                )
            })
            .collect::<Vec<CameraDevice>>();
        Ok(devices)
    }
}
