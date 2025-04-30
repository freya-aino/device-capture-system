use bincode::Encode;
use glob::glob;
use std::ffi::OsString;
use std::time::Duration;

use anyhow::Ok;
use anyhow::{Error, Result};
use cpal::traits::HostTrait;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig, SupportedStreamConfigRange};

#[cfg(target_os = "linux")]
use v4l::FourCC;
use v4l::frameinterval::FrameIntervalEnum;
use v4l::video::Capture;

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
            },
        };

        let sample_format = sample_format.unwrap_or(supported_config.sample_format());

        let stream = self
            .microphone
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

#[cfg(target_os = "linux")]
pub struct V4lCameraDevice {
    id: u16,
    name: String,
    device: v4l::Device,
    camera_configs: Vec<CameraConfig>,
}

#[derive(Debug)]
pub struct CameraConfig {
    width: u32,
    height: u32,
    fps: f32,
    fourcc: String,
}

#[cfg(target_os = "linux")]
impl V4lCameraDevice {
    pub fn new(id: u16) -> Self {
        let dev = v4l::Device::new(id as usize).unwrap();

        let caps = dev.query_caps().unwrap();

        // let configs = Format::new(width, height, fourcc);

        let formats = dev.enum_formats().unwrap();

        let mut out_configs = Vec::<CameraConfig>::new();

        for format in formats.iter() {
            let index = format.index;
            let flags = format.flags;
            // let description = format.description;
            let fourcc = format.fourcc;
            let fourcc_str = String::from_utf8(fourcc.repr.to_vec()).unwrap();

            let frame_sizes = dev.enum_framesizes(fourcc).unwrap();
            for fs in frame_sizes {
                let size = fs.size.to_discrete().into_iter().nth(0).unwrap();
                let width = size.width;
                let height = size.height;

                let frame_intervals = dev.enum_frameintervals(fourcc, width, height).unwrap();
                for fi in frame_intervals {
                    let fps = match fi.interval {
                        FrameIntervalEnum::Discrete(frac) => {
                            frac.denominator as f32 / frac.numerator as f32
                        }
                        FrameIntervalEnum::Stepwise(step) => {
                            step.step.denominator as f32 / step.step.numerator as f32
                        }
                    };

                    let cfg = CameraConfig {
                        width: size.width,
                        height: size.height,
                        fps: fps,
                        fourcc: fourcc_str.clone(),
                    };
                    out_configs.push(cfg);
                }
            }
        }

        V4lCameraDevice {
            id: id,
            name: caps.card,
            device: dev,
            camera_configs: out_configs,
        }
    }

    pub fn print_configurations(&self) -> Result<(), Error> {
        for cfg in &self.camera_configs {
            println!("{:?}", cfg);
        }
        Ok(())
    }
}

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
pub enum Device {
    Microphone(CpalMicrophoneDevice),
    Camera(V4lCameraDevice),
}

pub struct DeviceManager {
    pub system_id: u8,
    pub device_info: Device,
}

impl DeviceManager {
    pub fn get_all_available_cameras() -> Result<Vec<V4lCameraDevice>, Error> {
        let device_paths = all_devices_paths_linux().unwrap();
        println!("Found {} devices!", device_paths.len());
        if device_paths.len() == 0 {
            println!("No devices found!");
            return Ok(vec![]);
        }
        Ok(vec![])
        // let mut cam = CameraDevice::new(device_paths[0].clone(), device_paths[0].clone());
        // let confs = cam.get_all_available_configs()
        //     .unwrap()
        //     .iter()
        //     .map(|config| DeviceConfig::Camera(config.clone()))
        //     .collect::<Vec<DeviceConfig>>();
        // Ok(confs)
    }
}
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

// pub fn get_all_available_devices(
//     cpal_host: &cpal::Host,
// ) -> Result<Vec<Self>, Error> {
//     let mut all_device_infos = Vec::new();

//     all_device_infos.extend(
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
