use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{self, Duration, SystemTime};

use anyhow::Ok;
use anyhow::{Error, Result};
use bincode::{Decode, Encode};
use clap::ValueEnum;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::traits::HostTrait;
use cpal::StreamConfig;
use nokhwa::{Camera, NokhwaError};
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{CameraFormat, FrameFormat as PixelFormat};
use nokhwa::utils::RequestedFormat;
use nokhwa::utils::RequestedFormatType;
use nokhwa::utils::{ApiBackend, CameraIndex};
use tokio;
use tokio::sync::{Mutex, Notify};

use crate::FramePacket;

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

impl MicrophoneConfig {
    pub fn new(sample_rate: u32, channels: u8, sample_size: u16) -> Self {
        MicrophoneConfig {
            sample_rate,
            channels,
            sample_size,
        }
    }
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

#[derive(Debug, Clone, Encode, Decode)]
pub struct DeviceInformation {
    pub id: u8,
    pub name: String,
    pub device_type: DeviceType,
}

// impl DeviceInformation {
//     pub fn new(id: u8, name: String, device_type: DeviceType) -> DeviceInformation {
//         DeviceInformation {
//             id,
//             name,
//             device_type,
//         }
//     }

    // pub fn create_device(&self, cpal_host: Option<&cpal::Host>) -> Result<Box<DevicePrimitive>, Error> {
    //     match self.device_type {
    //         DeviceType::Camera => {
    //             Ok(Box::new(DevicePrimitive::Camera(self.create_camera()?)))
    //         },
    //         DeviceType::Microphone => {
    //             assert!(cpal_host.is_some(), "No cpal host provided for microphone device");
    //             Ok(Box::new(DevicePrimitive::Microphone(self.create_microphone(cpal_host.unwrap())?)))
    //         }
    //     }
    // }

    // fn create_microphone(&self, cpal_host: &cpal::Host) -> Result<cpal::Device, Error> {

    //     let mic = cpal_host
    //         .input_devices()
    //         .unwrap()
    //         .nth(self.id as usize)
    //         .ok_or_else(|| Error::msg("Device not found"));

    //     mic
    // }

    // fn create_camera(&self) -> Result<Camera, NokhwaError> {
    //     let camera = Camera::new(
    //         CameraIndex::Index(self.id as u32),
    //         RequestedFormat::new::<RgbAFormat>(RequestedFormatType::None),
    //     );
    //     camera
    // }
// }

struct CameraDevice {
    id: u8,
    name: String,
    cam: Option<Camera>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl CameraDevice {
    fn new(id: u8, name: String) -> Self {
        CameraDevice {
            id: id,
            name: name,
            cam: None,
            handle: None,
        }
    }

    fn initialize(&mut self, config: CameraConfig) -> Result<(), Error> {
        let camera = Camera::new(
            CameraIndex::Index(self.id as u32),
            RequestedFormat::new::<RgbAFormat>(
                RequestedFormatType::Closest(
                    CameraFormat::new_from(
                        config.width,
                        config.height,
                        PixelFormat::from(config.pixel_format),
                        config.fps,
                    ),
                )
            )
        )?;

        self.cam = Some(camera);

        Ok(())
    }

    // async fn start(&mut self, data_tx: flume::Sender<FramePacket>) -> Result<(), Error> {

    //     let mut camera = self.cam.take()
    //         .ok_or_else(||Error::msg("Camera not initialized"))?;

    //     camera.open_stream()?;

    //     let handle = tokio::spawn(async move {
    //         while camera.is_stream_open() {
    //             let frame = camera.frame().unwrap();
    //             let frame_packet = FramePacket::new(
    //                 SystemTime::now(),
    //                 DeviceInformation {
    //                     id: self.id,
    //                     name: self.name.clone(),
    //                     device_type: DeviceType::Camera,
    //                 },
    //                 vec![frame.resolution().x() as u16, frame.resolution().y() as u16],
    //                 frame.buffer().to_vec(),
    //             );

    //             if data_tx.send_async(frame_packet).await.is_err() {
    //                 println!("Error sending data to channel, stopping stream.");
    //                 break;
    //             }
    //         }
    //         camera.stop_stream().unwrap();
    //     });

    //     self.handle = Some(handle);

    //     Ok(())
    // }

    // fn stop(&mut self) -> Result<(), Error> {
    //     let cam = self.cam.as_mut().unwrap();
    //     cam.stop_stream()?;
    //     self.cam = None;
    //     Ok(())
    // }
}


pub struct MicrophoneDevice {
    id: u8,
    name: String,
    mic: Option<cpal::Device>,
    stream: Option<cpal::Stream>,
}

impl MicrophoneDevice {
    pub fn new(id: u8, name: String) -> Self {
        MicrophoneDevice {
            id: id,
            name: name,
            mic: None,
            stream: None,
        }
    }

    pub fn initialize(&mut self, cpal_host: &cpal::Host) -> Result<(), Error> {
        let mic = cpal_host
            .input_devices()
            .unwrap()
            .nth(self.id as usize)
            .ok_or_else(|| Error::msg("Device not found"))?;

        self.mic = Some(mic);

        Ok(())
    }

    pub fn start(&mut self, config: MicrophoneConfig, timeout: Option<Duration>, data_tx: flume::Sender<FramePacket>) -> Result<(), Error> {
        
        let mic = self.mic.as_ref().unwrap();
        let mic_conf = StreamConfig {
            channels: config.channels as u16,
            sample_rate: cpal::SampleRate(config.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };
        
        let device_info = DeviceInformation {
            id: self.id,
            name: self.name.clone(),
            device_type: DeviceType::Microphone,
        };

        let stream = mic.build_input_stream(
            &mic_conf,
            move |data: &[u8], _: &cpal::InputCallbackInfo| {
                let frame_packet = FramePacket::new(
                    SystemTime::now(),
                    device_info.clone(),
                    vec![config.sample_rate as u16, config.channels as u16], // TODO: this is probably not correct, but we want to test the function first
                    data.to_vec(),
                );

                if data_tx.send(frame_packet).is_err() {
                    println!("Error sending data to channel, stopping stream.");
                    return;
                }
            },
            move |err| {
                eprintln!("Error: {:?}", err);
            },
            timeout
        )?;

        stream.play()?;

        self.stream = Some(stream);

        Ok(())
    }
}


// --- device manager ---


pub struct DeviceManager {
    pub system_id: u8,
    pub device_info: DeviceInformation,
    pub status: Option<DeviceStatus>,
    shutdown: Arc<Notify>,
}


impl DeviceManager {

    // pub async fn run(&mut self, data_tx: flume::Sender<FramePacket>) -> Result<(), Error> {
        
    //     let device = match self.device_info.device_type {
    //         DeviceType::Camera => self.create_device(None)?,
    //         DeviceType::Microphone => self.create_device(Some(&cpal::default_host()))?,
    //     };

    //     match device {
    //         DevicePrimitive::Camera(camera) => {
                
    //         },

    //         DevicePrimitive::Microphone(mic) => {
    //             let mut stream = mic.build_input_stream(
    //                 stream_config, 
    //                 data_callback, 
    //                 error_callback, 
    //                 timeout
    //             );

    //             // loop {
    //             //     let data = stream.read()?;
    //             //     data_tx.send_async(data).await?;
    //             // }
    //         }
    //     }

    //     Ok(())
    // }

    // fn initialize(&mut self, device: &mut DevicePrimitive, config: DeviceConfig) -> Result<(), Error> {
    //     match device {
    //         DevicePrimitive::Camera(camera) => {
    //             camera.
    //             camera.open_stream()?;
    //             self.status = Some(DeviceStatus::Initialized);
    //         },
    //         DevicePrimitive::Microphone(mic) => {
    //             // Initialize microphone stream here
    //             self.status = Some(DeviceStatus::Initialized);
    //         }
    //     }
    //     Ok(())
    // }

    // fn create_device(&mut self, cpal_host: Option<&cpal::Host>) -> Result<Box<DevicePrimitive>, Error> {
    //     let device = self.device_info.create_device(cpal_host)?;
    //     self.status = Some(DeviceStatus::Created);
    //     Ok(device)
    // }

    pub fn get_all_available_devices(nokhwa_backend: &ApiBackend, cpal_host: &cpal::Host) -> Result<Vec<Self>, Error> {
        
        let mut all_device_infos = Vec::new();

        all_device_infos.extend(nokhwa::query(*nokhwa_backend)
            .unwrap()
            .into_iter()
            .map(|device| {
                DeviceInformation {
                    id: device.index().as_index().unwrap() as u8,
                    name: device.human_name().to_string(),
                    device_type: DeviceType::Camera,
                }
            })
            .collect::<Vec<DeviceInformation>>()
        );

        all_device_infos.extend(cpal_host
            .input_devices()?
            .enumerate()
            .map(|(i, device)| {
                DeviceInformation {
                    id: i as u8,
                    name: device.name().unwrap(),
                    device_type: DeviceType::Microphone,
                }
            })
            .collect::<Vec<DeviceInformation>>()
        );

        let out = all_device_infos
            .iter()
            .enumerate()
            .map(|(i, device_info)| {
                Self {
                    system_id: i as u8,
                    device_info: device_info.clone(),
                    status: None,
                    shutdown: Arc::new(Notify::new()),
                }
            })
            .collect::<Vec<Self>>();
        
        Ok(out)
    }
}
