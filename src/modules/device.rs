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


trait DeviceConfig {}

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

impl DeviceConfig for CameraConfig {}
impl DeviceConfig for MicrophoneConfig {}


// --- device ---


#[derive(Debug, Clone, ValueEnum, PartialEq, Encode, Decode)]
pub enum DeviceType {
    Camera,
    Microphone,
}

pub trait Device {
    type Config: DeviceConfig;

    fn get_all_available_devices() -> Result<Vec<DeviceInformation>, Error>;

    fn new(id: u8, name: String) -> Self;

    fn get_available_configs(&self) -> Result<Vec<Self::Config>, Error>;
    
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
    device: Camera,
    // config: Option<CameraFormat>,
}

pub struct MicrophoneDevice {
    information: DeviceInformation,
    device: cpal::Device,
    // config: Option<cpal::StreamConfig>,
}

impl Device for CameraDevice {
    type Config = CameraConfig;

    fn new(id: u8, name: String) -> Self {
        
        let index = CameraIndex::Index(id as u32);
        let cam = Camera::new(index, RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestFrameRate)).unwrap();

        CameraDevice {
            information: DeviceInformation {
                id: id, 
                name: name, 
                device_type: DeviceType::Camera,
            },
            device: cam,
            // config: None,
        }
    }

    fn get_available_configs(&self) -> Result<Vec<Self::Config>, Error> {

        Ok(vec![])
    }

    fn start(&mut self) -> Result<(), Error> {
        // assert!(self.device.is_some(), "Trying to start without Device initialized");

        // let cam = self.device.as_ref().unwrap();

        // let config = CameraFormat::new_from(
        //     conf.width,
        //     conf.height,
        //     conf.pixel_format,
        //     conf.fps,
        // );
        
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

    type Config = MicrophoneConfig;

    fn new(id: u8, name: String) -> Self {
                
        let host = cpal::default_host();
        let device = host.input_devices()
            .unwrap()
            .nth(id as usize)
            .ok_or_else(|| Error::msg("Device not found"))
            .unwrap();

        
        // let config = SupportedStreamConfig::new(
        //     conf.channels as u16,
        //     cpal::SampleRate(conf.sample_rate),
        //     cpal::SupportedBufferSize::Unknown,
        //     cpal::SampleFormat::U8,
        // ).config();


        MicrophoneDevice {
            information: DeviceInformation { 
                id: id,
                name: name,
                device_type: DeviceType::Microphone,
            },
            device: device,
        }
    }

    fn start(&mut self) -> Result<(), Error> {

        // assert!(self.device.is_some(), "Trying to start without Device initialized");

        // let device = self.device.as_ref().unwrap();


        Ok(())
    }
    
    fn get_available_configs(&self) -> Result<Vec<Self::Config>, Error> {
        let host = cpal::default_host();
        let supported_configs = host.input_devices()?
            .nth(self.information.id as usize)
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
}


// --- device manager ---


pub enum ManagedDevice {
    Camera(CameraDevice),
    Microphone(MicrophoneDevice),
}

impl ManagedDevice {

    fn get_available_configs(&self) -> Result<Vec<Box<dyn DeviceConfig>>, Error> {
        match self {
            ManagedDevice::Camera(camera) => Ok(camera.get_available_configs()?.into_iter().map(|c| Box::new(c) as Box<dyn DeviceConfig>).collect()),
            ManagedDevice::Microphone(mic) => Ok(mic.get_available_configs()?.into_iter().map(|c| Box::new(c) as Box<dyn DeviceConfig>).collect()),
        }
    }

    fn start(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum DeviceStatus {
    Created,
    Initialized,
    Started,
    Paused,
    Stopped,
    Terminated,
}

pub struct DeviceManager {
    pub system_id: u8,
    pub device: ManagedDevice,
    // pub device_info: DeviceInformation,
    // pub config_id: Option<u8>,
    pub status: DeviceStatus,
}

impl DeviceManager {
    pub fn new(device_info: DeviceInformation, system_id: u8) -> Self {
        DeviceManager {
            // device_info: device_info,
            // config_id: None,
            device: match device_info.device_type {
                DeviceType::Camera => ManagedDevice::Camera(CameraDevice::new(system_id, device_info.name)),
                DeviceType::Microphone => ManagedDevice::Microphone(MicrophoneDevice::new(system_id, device_info.name)),
            },
            system_id: system_id,
            status: DeviceStatus::Created,
        }
    }

    pub fn get_all_available_devices_managed() -> Result<Vec<Self>, Error> {
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

