use anyhow::Ok;
use anyhow::{Error, Result};
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::SupportedStreamConfig;
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::CameraFormat;
use nokhwa::utils::CameraIndex;
use clap::ValueEnum;
use bincode::{Encode, Decode};
use nokhwa::utils::RequestedFormat;
use nokhwa::utils::RequestedFormatType;
use nokhwa::Camera;
use nokhwa::utils::FrameFormat as PixelFormat;

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
    channels: u32,
    sample_size: u32,
}

pub enum DeviceConfig {
    Camera(CameraConfig),
    Microphone(MicrophoneConfig),
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
pub trait Device {

    fn get_all_available_devices() -> Result<Vec<DeviceInformation>, Error>;
    fn get_all_available_configs(&mut self) -> Result<Vec<DeviceConfig>, Error>;

    fn new(id: u8, name: String) -> Self;
    
    fn instantiate_device(&mut self) -> Result<(), Error>;
    fn start(&mut self) -> Result<(), Error>;
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct DeviceInformation {
    pub id: u8,
    pub name: String,
    pub device_type: DeviceType,
}

pub struct CameraDevice {
    information: DeviceInformation,
    device: Option<Camera>,
    status: DeviceStatus,
}

pub struct MicrophoneDevice {
    information: DeviceInformation,
    device: Option<cpal::Device>,
    status: DeviceStatus,
}

impl Device for CameraDevice {

    fn new(id: u8, name: String) -> Self {
        CameraDevice {
            information: DeviceInformation {
                id: id, 
                name: name, 
                device_type: DeviceType::Camera,
            },
            device: None,
            status: DeviceStatus::Created,
        }
    }

    fn get_all_available_configs(&mut self) -> Result<Vec<DeviceConfig>, Error> {

        assert!(self.device.is_some(), "Trying to get configs without Device initialized");
        assert!(self.status != DeviceStatus::Created, "UNEXPECTED: Trying to get configs while state is 'created', this should always be false.");


        let cam = self.device.as_mut().unwrap();

        let formats = cam.compatible_camera_formats()?;

        let mut cam_formats = formats
            .into_iter()
            .map(|conf| {
                CameraConfig {
                    width: conf.width(),
                    height: conf.height(),
                    fps: conf.frame_rate(),
                    pixel_format: conf.format(),
                }
            })
            .collect::<Vec<CameraConfig>>();

        cam_formats.sort_by_key(|conf| (conf.width * conf.height, conf.fps));

        Ok(cam_formats
            .into_iter()
            .map(|conf| DeviceConfig::Camera(conf))
            .collect::<Vec<DeviceConfig>>()
        )
    }

    // let config = CameraFormat::new_from(
    //     conf.width,
    //     conf.height,
    //     conf.pixel_format,
    //     conf.fps,
    // );

    fn instantiate_device(&mut self) -> Result<(), Error> {
        let cam = Camera::new(
            CameraIndex::Index(self.information.id as u32), 
            RequestedFormat::new::<RgbAFormat>(RequestedFormatType::None),
        ).unwrap();
        self.device = Some(cam);
        self.status = DeviceStatus::Initialized;
        Ok(())
    }

    fn start(&mut self) -> Result<(), Error> {
        // assert!(self.device.is_some(), "Trying to start without Device initialized");
        Ok(())
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
}

impl Device for MicrophoneDevice {

    fn new(id: u8, name: String) -> Self {
        MicrophoneDevice { 
            information: DeviceInformation { 
                id: id,
                name: name,
                device_type: DeviceType::Microphone,
            },
            device: None,
            status: DeviceStatus::Created,
        }
    }


    fn get_all_available_configs(&mut self) -> Result<Vec<DeviceConfig>, Error> {
        
        assert!(self.device.is_some(), "Trying to get configs without Device initialized");
        assert!(self.status != DeviceStatus::Created, "UNEXPECTED: Trying to get configs while state is 'created', this should always be false.");

        let mic = self.device.as_ref().unwrap();
        let supported_configs = mic.supported_input_configs()?;

        let mut out_configs = Vec::new();
        for conf in supported_configs {
            out_configs.push(MicrophoneConfig {
                sample_rate: conf.min_sample_rate().0,
                channels: conf.channels() as u32,
                sample_size: conf.sample_format().sample_size() as u32,
            });
        }

        out_configs.sort_by_key(|conf| (conf.sample_rate, conf.sample_size, conf.channels));

        Ok(out_configs
            .into_iter()
            .map(|conf| DeviceConfig::Microphone(conf))
            .collect::<Vec<DeviceConfig>>()
        )
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
    

    fn instantiate_device(&mut self) -> Result<(), Error> {
        let host = cpal::default_host();
        let device = host.input_devices()
            .unwrap()
            .nth(self.information.id as usize)
            .ok_or_else(|| Error::msg("Device not found"))
            .unwrap();
        self.device = Some(device);
        self.status = DeviceStatus::Initialized;
        Ok(())
    }

    fn start(&mut self) -> Result<(), Error> {
        Ok(())
    }
}


// --- device manager ---

pub enum ManagedDevice {
    Camera(CameraDevice),
    Microphone(MicrophoneDevice),
}

impl ManagedDevice {

    pub fn new(device_info: DeviceInformation) -> Self {
        match device_info.device_type {
            DeviceType::Camera => ManagedDevice::Camera(CameraDevice::new(device_info.id, device_info.name)),
            DeviceType::Microphone => ManagedDevice::Microphone(MicrophoneDevice::new(device_info.id, device_info.name))
        }
    }

    // fn get_all_available_configs(&self) -> Result<Vec<DeviceConfig>, Error> {
    //     match self {
    //         ManagedDevice::Camera(_) => self.get_all_available_configs(),
    //         ManagedDevice::Microphone(_) => self.get_all_available_configs(),
    //     }
    // }
}


pub struct DeviceManager {
    pub system_id: u8,
    pub device: ManagedDevice,
}

impl DeviceManager {

    pub fn new(device_info: DeviceInformation, system_id: u8) -> Self {
        DeviceManager {
            device: ManagedDevice::new(device_info),
            system_id: system_id,
        }
    }

    pub fn get_all_available_devices() -> Result<Vec<Self>, Error> {
        let mut devices = Vec::new();
        devices.extend(CameraDevice::get_all_available_devices()?);
        devices.extend(MicrophoneDevice::get_all_available_devices()?);

        let managed_devices = devices
            .iter()
            .enumerate()
            .map(|(i, device)| {
                DeviceManager::new(device.clone(), i as u8)
            })
            .collect::<Vec<DeviceManager>>();

        Ok(managed_devices)
    }

}

