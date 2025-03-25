use anyhow::{Error, Result};
use std::sync::Arc;
use nokhwa::utils::FrameFormat as PixelFormat;


#[derive(Debug)]
enum DeviceType {
    Camera,
    Microphone,
}

#[derive(Debug)]
pub enum DeviceStatus {
    Initialized,
    Available,
    Capturing,
    Paused,
    Error,
    Disabled,
}

// ---

trait DeviceConfig {}

trait Device {
    type Config: DeviceConfig;

    fn new(id: u32, name: String, hardware_string: Option<String>) -> Self;
    fn id(&self) -> u32;
    fn name(&self) -> String;
    fn hardware_string(&self) -> Option<String>;
    fn get_available_configs(&self) -> Result<Arc<[Self::Config]>, Error>;
}

// ---

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
pub struct CameraDevice {
    id: u32,
    name: String,
    hardware_string: Option<String>,
}

#[derive(Debug)]
pub struct MicrophoneDevice {
    id: u32,
    name: String,
    hardware_string: Option<String>,
}

// ---

impl DeviceConfig for CameraConfig {}

impl DeviceConfig for MicrophoneConfig {}

impl Device for CameraDevice {
    type Config = CameraConfig;

    fn new(id: u32, name: String, hardware_string: Option<String>) -> Self {
        CameraDevice {
            id: id,
            name: name,
            hardware_string: hardware_string,
        }
    }
    fn id(&self) -> u32 { self.id }
    fn name(&self) -> String { self.name.clone() }
    fn hardware_string(&self) -> Option<String> { self.hardware_string.clone() }
    fn get_available_configs(&self) -> Result<Arc<[Self::Config]>, Error> {
        todo!("Implement get all configs for camera");
        Ok(Arc::new([]))
    }
}

impl Device for MicrophoneDevice {
    type Config = MicrophoneConfig;

    fn new(id: u32, name: String, hardware_string: Option<String>) -> Self {
        MicrophoneDevice {
            id: id,
            name: name,
            hardware_string: hardware_string,
        }
    }
    fn id(&self) -> u32 { self.id }
    fn name(&self) -> String { self.name.clone() }
    fn hardware_string(&self) -> Option<String> { self.hardware_string.clone() }
    fn get_available_configs(&self) -> Result<Arc<[Self::Config]>, Error> {
        todo!("Implement get all configs for microphone");
        Ok(Arc::new([]))
    }
}



// ---

#[derive(Debug)]
pub struct DeviceManager<D: Device> {
    device: D,
    configs: Arc<[D::Config]>,
    active_config: Option<usize>,
    status: DeviceStatus,
}

impl<D: Device> DeviceManager<D> {
    pub fn new(device: D) -> Result<Self, Error> {
        let configs = device.get_available_configs()?;
        Ok(DeviceManager {
            device: device,
            configs: configs,
            active_config: None,
            status: DeviceStatus::Initialized,
        })
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