use anyhow::{Error, Result};
use shared::{CameraConfig, Device, DeviceInformation, DeviceType};

use v4l::frameinterval::FrameIntervalEnum;
use v4l::video::Capture;

pub struct V4lCameraDevice {
    pub device_info: DeviceInformation,
    pub device: v4l::Device,
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
            let fourcc_str = String::from_utf8(fourcc.repr.to_vec()).unwrap();

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
                        fourcc: fourcc_str.clone(),
                    };
                    out_configs.push(cfg);
                }
            }
        }

        Ok(out_configs)
    }

    fn open(
        &mut self,
        config: Self::Config,
        timeout: Option<std::time::Duration>,
    ) -> std::result::Result<(), Error> {
        Ok(())
    }

    fn close(&mut self) -> std::result::Result<(), Error> {
        Ok(())
    }
}
