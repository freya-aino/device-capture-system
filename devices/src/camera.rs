use std::thread::{JoinHandle, spawn};

use anyhow::{Error, Result};
use shared::{
    CameraConfig, Device, DeviceInformation, DeviceType, FramePacket, FramePacketInformation,
};
use v4l::FourCC;
use v4l::buffer::Type;
use v4l::frameinterval::FrameIntervalEnum;
use v4l::io::mmap::Stream;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;

pub struct V4lCameraDevice {
    pub device_info: DeviceInformation,
    pub device: v4l::Device,
    pub handle: Option<JoinHandle<Result<(), Error>>>,
}

impl V4lCameraDevice {
    pub fn new(path: &str, id: u16) -> Self {
        let dev = v4l::Device::with_path(path).unwrap();
        let caps = dev.query_caps().unwrap();
        let device_info = DeviceInformation {
            id: id,
            name: caps.card,
            device_type: DeviceType::Camera,
        };
        V4lCameraDevice {
            device_info: device_info,
            device: dev,
            handle: None,
        }
    }
}

impl Device for V4lCameraDevice {
    type Config = CameraConfig;

    fn id(&self) -> u16 {
        self.device_info.id
    }

    fn name(&self) -> &str {
        &self.device_info.name
    }

    fn device_type(&self) -> &DeviceType {
        &self.device_info.device_type
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
                        fourcc: fourcc_byte.clone(),
                    };
                    out_configs.push(cfg);
                }
            }
        }

        Ok(out_configs)
    }

    fn open(
        &mut self,
        config: CameraConfig,
        callback: Box<dyn Fn(FramePacket) + Send + 'static>,
        timeout: Option<std::time::Duration>,
    ) -> Result<(), Error> {
        assert!(
            self.handle.is_none(),
            "Camera is already open (handle is not None)"
        );

        let mut format = self.device.format().expect("Failed to get device format");

        let buffer_count = 4;

        format.width = config.width;
        format.height = config.height;
        format.fourcc = FourCC::new(&config.fourcc);

        let format = self
            .device
            .set_format(&format)
            .expect("Failed to set device format");

        println!(
            "Config used for camera - {:?} - {:?}",
            self.device_info.name, format
        );

        let mut stream = Stream::with_buffers(&mut self.device, Type::VideoCapture, buffer_count)
            .expect("Failde to create Stream");

        if let Some(timeout) = timeout {
            stream.set_timeout(timeout);
        }

        println!("Opened camera device");

        let di = self.device_info.clone();

        let handle: JoinHandle<Result<(), Error>> = spawn(move || {
            for _ in 0..100 {
                // test wise TODO replace with handler
                let (buf, _) = stream.next().expect("Unable to read frame");
                let frame_shape = vec![format.width, format.height, 3];

                let frame_packet = FramePacket::new(
                    FramePacketInformation {
                        device_info: di.clone(),
                        rx_timestamp: None,
                        tx_timestamp: None,
                        frame_shape: frame_shape,
                    },
                    buf.to_vec().into_boxed_slice(),
                );

                callback(frame_packet);
            }
            Ok(())
        });

        self.handle = Some(handle);

        Ok(())
    }

    fn close(&mut self) -> std::result::Result<(), Error> {
        Ok(())
    }
}
