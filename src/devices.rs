use anyhow::{Error, Result};
use std::{sync::Arc, time::Instant};
use nokhwa::utils::CameraInfo;
use nokhwa::utils::FrameFormat as PixelFormat;

#[derive(Debug, Clone)]
enum DeviceType {
    Camera,
    Microphone,
}

#[derive(Debug)]
enum DeviceStatus {
    Initialized,
    Available,
    Capturing,
    Paused,
    Error,
    Disabled,
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

    fn get_all_available_devices() -> Result<Arc<[DeviceInformation]>, Error>;

    fn new(id: u32, name: String) -> Self;
    fn get_info(&self) -> DeviceInformation;
    // fn get_config(&self) -> Self::Config;

    fn get_available_configs(&self) -> Result<Arc<[Self::Config]>, Error>;
    
    // fn start(&self) -> Result<(), Error>;
    // fn stop(&self) -> Result<(), Error>;
    // fn pause(&self) -> Result<(), Error>;
    // fn resume(&self) -> Result<(), Error>;
}

// ---

#[derive(Debug, Clone)]
pub struct DeviceInformation {
    pub ID: u8,
    pub Name: String,
    pub DeviceType: DeviceType,
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

// #[derive(Debug)]
// pub struct FrameData {
//     frame: Vec<u8>,
//     timestamp: Instant,
// }

// ---

impl DeviceConfig for CameraConfig {}
impl DeviceConfig for MicrophoneConfig {}

impl Device for CameraDevice {
    type Config = CameraConfig;

    fn new(id: u32, name: String) -> Self {
        CameraDevice(DeviceInformation {
            ID: id as u8,
            Name: name,
            DeviceType: DeviceType::Camera,
        })
    }
    fn get_info(&self) -> DeviceInformation { self.0.clone() }

    fn get_available_configs(&self) -> Result<Arc<[Self::Config]>, Error> {
        todo!("Implement get all configs for camera");
    }
    fn get_all_available_devices() -> Result<Arc<[DeviceInformation]>, Error> {
        let nokhwa_backend = nokhwa::native_api_backend().unwrap();
        let devices = nokhwa::query(nokhwa_backend)
                .unwrap()
                .into_iter()
                .map(|device| {
                    DeviceInformation {
                        ID: device.index().as_index().unwrap() as u8,
                        Name: device.human_name().to_string(),
                        DeviceType: DeviceType::Camera,
                    }
                })
                .collect::<Arc<[DeviceInformation]>>();
        Ok(devices)
    }
    // fn start(&self) -> Result<(), Error> {
    //     todo!("Implement start capturing from the camera");
    // }
    // fn stop(&self) -> Result<(), Error> {
    //     todo!("Implement stop capturing from the camera");
    // }
    // fn pause(&self) -> Result<(), Error> {
    //     todo!("Implement pause capturing from the camera");
    // }
    // fn resume(&self) -> Result<(), Error> {
    //     todo!("Implement resume capturing from the camera");
    // }
    // fn get_current_frame(&self) -> Result<Self::Data, Error> {
    //     todo!("Implement get current frame from the camera");
    // }
}

// impl Device for MicrophoneDevice {
//     type Config = MicrophoneConfig;

//     fn new(id: u32, name: String) -> Self {
//         MicrophoneDevice { id: id, name: name }
//     }
//     fn name(&self) -> String { self.name.clone() }
//     fn id(&self) -> u32 { self.id }
//     fn get_available_configs(&self) -> Result<Arc<[Self::Config]>, Error> {
//         let host = cpal::default_host();
//         let supported_configs = device.supported_input_configs()?;
//         let mut out_configs = Vec::new();
//         for conf in supported_configs {
//             out_configs.push(MicrophoneConfig {
//                 sample_rate: conf.max_sample_rate().0 as u32,
//                 channels: conf.channels() as u32,
//                 sample_size: conf.sample_format().sample_size() as u32,
//             });
//         }
//         Ok(Arc::from(out_configs))
//     }
//     fn get_all_available_devices() -> Result<Vec<Self>, Error> {
//         let host = cpal::default_host();
//         let devices = host.input_devices()?;
//         let mut out_devices = Vec::new();
//         for (i, device) in devices.enumerate() {
//             out_devices.push(MicrophoneDevice::new(
//                 i as u32,
//                 device.name().unwrap_or_else(|| "Unknown".to_string()),
//             ));
//         }
//         Ok(out_devices)
//     }

//     fn new(id: u32, name: String) -> Self {
//         MicrophoneDevice { id: id, name: name }
//     }
//     fn id(&self) -> u32 { self.id }
//     fn name(&self) -> String { self.name.clone() }
//     fn get_available_configs(&self) -> Result<Arc<[Self::Config]>, Error> {
//         todo!("Implement get all configs for microphone");
//         Ok(Arc::new([]))
//     }
// }


// ---

// #[derive(Debug)]
// pub struct DeviceManager<D: Device> {
//     device: D,
//     configs: Arc<[D::Config]>,
//     active_config: Option<usize>,
//     status: DeviceStatus,
// }

// impl<D: Device> DeviceManager<D> {
//     pub fn new(device: D) -> Result<Self, Error> {
//         let configs = device.get_available_configs()?;
//         Ok(DeviceManager {
//             device: device,
//             configs: configs,
//             active_config: None,
//             status: DeviceStatus::Initialized,
//         })
//     }
// }

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