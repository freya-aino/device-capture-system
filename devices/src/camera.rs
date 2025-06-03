use std::ffi::OsString;
use std::thread::{JoinHandle, spawn};
use std::time::Instant;

use anyhow::{Error, Result, anyhow};
use flume::Sender;
use glob::glob;
use shared::{
    CameraConfig, Device, DeviceCommand, DeviceInformation, DeviceStatus, DeviceType, FramePacket,
    FramePacketInformation,
};
use v4l::buffer::Type;
use v4l::frameinterval::FrameIntervalEnum;
use v4l::{FourCC, Fraction};
// use v4l::video::capture::Parameters;
// use v4l::io::traits::{CaptureStream, OutputStream};
use v4l::video::Capture;

pub struct V4lCameraDevice {
    pub device_info: DeviceInformation,
    pub device: v4l::Device,
    pub command_tx: Option<Sender<DeviceCommand>>,
}

impl V4lCameraDevice {
    pub fn new(system_path: String) -> Result<Self, Error> {
        let dev = v4l::Device::with_path(&system_path).unwrap();
        let caps = dev.query_caps().unwrap();
        Ok(V4lCameraDevice {
            device_info: DeviceInformation {
                id: system_path,
                name: caps.card,
                device_type: DeviceType::Camera,
                device_status: DeviceStatus::Available,
            },
            device: dev,
            command_tx: None,
        })
    }

    pub fn get_all_v4l_devices() -> Result<Vec<V4lCameraDevice>, Error> {
        let device_paths = glob("/dev/video*")?
            .filter_map(Result::ok)
            .map(|path| path.into_os_string())
            .collect::<Vec<OsString>>();

        let mut devices = Vec::<V4lCameraDevice>::new();
        for dp in device_paths.iter() {
            devices.push(match dp.clone().into_string() {
                Ok(str_path) => V4lCameraDevice::new(str_path)?,
                Err(os_path) => V4lCameraDevice::new(os_path.to_string_lossy().to_string())?,
            })
        }
        Ok(devices)
    }
}

impl Device for V4lCameraDevice {
    type Config = CameraConfig;

    fn id(&self) -> &str {
        &self.device_info.id
    }
    fn name(&self) -> &str {
        &self.device_info.name
    }
    fn device_type(&self) -> &DeviceType {
        &self.device_info.device_type
    }
    fn device_status(&self) -> &DeviceStatus {
        &self.device_info.device_status
    }

    fn get_configs(&self) -> Result<Vec<CameraConfig>, Error> {
        let formats = self.device.enum_formats().unwrap();

        let mut out_configs = Vec::<CameraConfig>::new();

        for format in formats.iter() {
            // let index = format.index;
            // let flags = format.flags;
            // let description = format.description;
            let fourcc = format.fourcc;
            let fourcc_byte = fourcc.repr;

            // if fourcc != FourCC::new(b"YUYV") {
            // && fourcc != FourCC::new(b"NV12") {
            // continue;
            // }

            let frame_sizes = self.device.enum_framesizes(fourcc).unwrap();
            for fs in frame_sizes {
                let size = fs.size.to_discrete().into_iter().nth(0).unwrap();
                let width = size.width;
                let height = size.height;

                let frame_intervals = self
                    .device
                    .enum_frameintervals(fourcc, width, height)
                    .unwrap();
                for fi in frame_intervals {
                    let (num, denom) = match fi.interval {
                        FrameIntervalEnum::Discrete(frac) => {
                            (frac.numerator as u32, frac.denominator as u32)
                        }
                        FrameIntervalEnum::Stepwise(step) => {
                            (step.step.numerator as u32, step.step.denominator as u32)
                        }
                    };

                    let cfg = CameraConfig {
                        width: size.width,
                        height: size.height,
                        fps: (num, denom),
                        fourcc: fourcc_byte.map(|b| b as char),
                    };
                    out_configs.push(cfg);
                }
            }
        }

        Ok(out_configs)
    }

    fn start(
        &mut self,
        config: CameraConfig,
        callback: Box<dyn Fn(FramePacket) -> Result<(), Error> + Send + 'static>,
    ) -> Result<JoinHandle<Result<(), Error>>, Error> {
        assert!(
            self.device_status() == &DeviceStatus::Available,
            "Camera is not in available status"
        );

        let mut format = self.device.format().expect("Failed to get device format");
        format.width = config.width;
        format.height = config.height;
        format.fourcc = FourCC::new(&config.fourcc.map(|a| a as u8));

        let mut params = self.device.params().expect("Failed to get device params");
        params.interval = Fraction::new(config.fps.0, config.fps.1);

        println!(
            "Desired camera configurations - {:?}\n{:?}\n{:?}\n",
            self.device_info.name, params, format
        );

        let params_ = self
            .device
            .set_params(&params)
            .expect("Failed to set device params");

        let format_ = self
            .device
            .set_format(&format)
            .expect("Failed to set device format");

        println!(
            "Config used by camera - {:?}\n{:?}\n{:?}\n",
            self.device_info.name,
            self.device.format(),
            self.device.params()
        );

        let mut stream =
            v4l::io::mmap::Stream::with_buffers(&mut self.device, Type::VideoCapture, 128)
                .expect("Failde to create Stream");

        // if let Some(timeout) = timeout {
        //     stream.set_timeout(timeout);
        // }

        println!("Camera device started");

        let device_information = self.device_info.clone();

        let (tx, rx) = flume::unbounded::<DeviceCommand>();
        self.command_tx = Some(tx);

        let handle = spawn(move || -> Result<(), Error> {
            loop {
                let timing = Instant::now();

                match rx.try_recv() {
                    Err(err) => match err {
                        flume::TryRecvError::Empty => {}
                        flume::TryRecvError::Disconnected => break,
                    },
                    Ok(command) => match command {
                        DeviceCommand::Stop => break,
                        _ => {}
                    },
                }

                let (buf, _) = v4l::io::traits::CaptureStream::next(&mut stream)
                    .expect("Unable to read frame");
                let frame_shape = vec![config.width, config.height];

                let frame_packet = FramePacket::new(
                    FramePacketInformation {
                        device_info: device_information.clone(),
                        rx_timestamp: None,
                        tx_timestamp: None,
                        frame_shape: frame_shape,
                    },
                    buf.to_vec().into_boxed_slice(),
                );

                callback(frame_packet)?;

                let elapsed = timing.elapsed();
                println!("Frame capture took: {:?}", elapsed);
            }

            v4l::io::traits::Stream::stop(&mut stream)?;
            Ok(())
        });

        Ok(handle)
    }

    fn stop(&mut self) -> Result<(), Error> {
        match self.command_tx.take() {
            Some(tx) => {
                tx.send(DeviceCommand::Stop).unwrap();
                Ok(())
            }
            None => Err(anyhow!("tryed closing without tx initialized")),
        }
    }
}
