use glob::glob;
use std::collections::HashSet;
use std::ffi::OsString;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Ok;
use anyhow::{Error, Result};
use bincode::{config, Decode, Encode};
use clap::ValueEnum;
use cpal::traits::HostTrait;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{InputCallbackInfo, SampleFormat, StreamConfig, SupportedStreamConfig, SupportedStreamConfigRange};
use tokio;

use crate::FramePacket;

#[cfg(target_os = "linux")]
use v4l::Device;

#[cfg(target_os = "linux")]
fn all_devices_paths_linux() -> Result<Vec<OsString>, Error> {
    use std::ffi::OsString;

    let device_paths = glob("/dev/video*")?
        .filter_map(Result::ok)
        .map(|path| path.into_os_string())
        .collect::<Vec<OsString>>();
    Ok(device_paths)
}

pub struct CpalMicrophoneDevice {
    id: u16,
    name: String,
    microphone: cpal::Device,
    microphone_configs: Vec<SupportedStreamConfigRange>,
    stream: Option<cpal::Stream>,
}

impl CpalMicrophoneDevice {
    pub fn new(id: u16, cpal_host: &cpal::Host) -> Self {
        let mic = cpal_host
            .input_devices()
            .unwrap()
            .nth(id as usize)
            .ok_or_else(|| Error::msg("Device not found"))
            .unwrap();

        let conf = mic
            .supported_input_configs()
            .unwrap()
            .collect::<Vec<SupportedStreamConfigRange>>();

        CpalMicrophoneDevice {
            id: id,
            name: mic.name().unwrap(),
            microphone: mic,
            microphone_configs: conf,
            stream: None,
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn print_configurations(&self) {
        for (i, conf) in self.microphone_configs.iter().enumerate() {
            println!("Microphone: {} - {:?}", i, conf);
        }
    }

    pub fn open(
        &mut self,
        timeout: Option<Duration>,
        sample_rate: Option<u32>, 
        channels: Option<u16>,
        buffer_size: Option<u32>,
        sample_format: Option<SampleFormat>,
    ) -> Result<(), Error> {

        let supported_config = self.microphone.default_input_config().unwrap();

        let stream_config = StreamConfig {
            channels: channels.unwrap_or(supported_config.channels()),
            sample_rate: match sample_rate {
                Some(rate) => cpal::SampleRate(rate),
                None => supported_config.sample_rate(),
            },
            buffer_size: match buffer_size {
                Some(size) => cpal::BufferSize::Fixed(size),
                None => cpal::BufferSize::Default,
            }
        };

        let sample_format = sample_format.unwrap_or(
            supported_config.sample_format()
        );

        let stream = self.microphone
            .build_input_stream_raw(
                &stream_config,
                sample_format,
                |data, info| {
                    // process audio data
                    let bytes = data.bytes();

                    println!("Received {} bytes of audio data", bytes.len());

                    // TODO: process audio data here
                },
                |err| {
                    println!("Error from audio stream: {}", err);
                },
                timeout,
            )
            .unwrap();

        stream.play().unwrap();

        self.stream = Some(stream);

        // TODO: listen to control signal here or persist stream

        Ok(())
    }

    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(stream) = self.stream.take() {
            stream.pause()?;
        }
        self.stream = None;
        Ok(())
    }
}

pub enum Device {
    Microphone(CpalMicrophoneDevice),
}

// impl CameraDevice {
//     pub fn new(device_information: DeviceInformation) -> Self {
//         CameraDevice {
//             device_information: device_information.clone(),
//         }
//     }
// }

// pub struct CameraDevice {
//     id: u8,
//     name: String,
//     handle: Option<tokio::task::JoinHandle<()>>,
// }

// impl MicrophoneDevice {
//     pub fn new(id: u8, name: String) -> Self {
//         MicrophoneDevice {
//             id: id,
//             name: name,
//             mic: None,
//             stream: None,
//         }
//     }

//     pub fn start(
//         &mut self,
//         config: MicrophoneConfig,
//         timeout: Option<Duration>,
//         data_tx: flume::Sender<FramePacket>,
//     ) -> Result<(), Error> {

//         let device_info = DeviceInformation {
//             id: self.id,
//             name: self.name.clone(),
//             device_type: DeviceType::Microphone,
//         };

//         let stream = mic.build_input_stream(
//             &mic_conf,
//             move |data: &[u8], _: &cpal::InputCallbackInfo| {
//                 let frame_packet = FramePacket::new(
//                     SystemTime::now(),
//                     device_info.clone(),
//                     vec![config.sample_rate as u16, config.channels as u16], // TODO: this is probably not correct, but we want to test the function first
//                     data.to_vec(),
//                 );

//                 if data_tx.send(frame_packet).is_err() {
//                     println!("Error sending data to channel, stopping stream.");
//                     return;
//                 }
//             },
//             move |err| {
//                 eprintln!("Error: {:?}", err);
//             },
//             timeout,
//         )?;

//         stream.play()?;

//         self.stream = Some(stream);

//         Ok(())
//     }
// }

// #[derive(Debug)]
// pub struct CameraConfig {
//     width: u32,
//     height: u32,
//     fps: u32,
//     pixel_format: PixelFormat,
// }

// #[derive(Debug)]
// pub struct MicrophoneConfig {
//     min_sample_rate: u32,
//     max_sample_rate: u32,
//     channels: u8,
//     sample_size: u16,
// }

// #[derive(Debug)]
// pub enum DeviceConfig {
//     Camera(CameraFormat),
//     Microphone(cpal::SupportedStreamConfigRange),
// }

// --- device ---

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

// pub fn start(&mut self, config: CameraFormat) -> Result<(), Error> {
//     let cam = self.cam.as_mut().unwrap();

//     let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(config));
//     let r = cam.set_camera_requset(format)?;

//     Ok(())
// }

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
// }

// --- device manager ---

// pub struct DeviceManager {
//     pub system_id: u8,
//     pub device_info: DeviceInformation,
// }

// impl DeviceManager {
// pub fn get_all_available_configs(&self) -> Result<Vec<DeviceConfig>, Error> {
//     match self.device_info.device_type {
//         DeviceType::Camera => {
//             let mut cam = CameraDevice::new(self.device_info.id, self.device_info.name.clone());
//             let confs = cam.get_all_available_configs()
//                 .unwrap()
//                 .iter()
//                 .map(|config| DeviceConfig::Camera(config.clone()))
//                 .collect::<Vec<DeviceConfig>>();
//             Ok(confs)
//         },
//         DeviceType::Microphone => {
//             let mut mic = MicrophoneDevice::new(self.device_info.id, self.device_info.name.clone());
//             let confs = mic.get_all_available_configs()
//                 .unwrap()
//                 .iter()
//                 .map(|config| DeviceConfig::Microphone(config.clone()))
//                 .collect::<Vec<DeviceConfig>>();
//             Ok(confs)
//         },
//     }
// }

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

// pub fn get_all_available_devices(
//     nokhwa_backend: &ApiBackend,
//     cpal_host: &cpal::Host,
// ) -> Result<Vec<Self>, Error> {
//     let mut all_device_infos = Vec::new();

//     all_device_infos.extend(
//         nokhwa::query(*nokhwa_backend)
//             .unwrap()
//             .into_iter()
//             .map(|device| DeviceInformation {
//                 id: device.index().as_index().unwrap() as u8,
//                 name: device.human_name().to_string(),
//                 device_type: DeviceType::Camera,
//             })
//             .collect::<Vec<DeviceInformation>>(),
//     );

//     all_device_infos.extend(
//         cpal_host
//             .input_devices()?
//             .enumerate()
//             .map(|(i, device)| DeviceInformation {
//                 id: i as u8,
//                 name: device.name().unwrap(),
//                 device_type: DeviceType::Microphone,
//             })
//             .collect::<Vec<DeviceInformation>>(),
//     );

//     let out = all_device_infos
//         .iter()
//         .enumerate()
//         .map(|(i, device_info)| Self {
//             system_id: i as u8,
//             device_info: device_info.clone(),
//             status: None,
//         })
//         .collect::<Vec<Self>>();

//     Ok(out)
// }
// }
