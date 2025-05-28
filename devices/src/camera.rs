use anyhow::{Error, Result};
use glob::glob;
use shared::{CameraConfig, DeviceInformation, DeviceType};
use std::ffi::OsString;

use v4l::frameinterval::FrameIntervalEnum;
use v4l::video::Capture;

pub fn get_all_camera_devices_linux() -> Result<Vec<V4lCameraDevice>, Error> {
    let device_paths = glob("/dev/video*")?
        .filter_map(Result::ok)
        .map(|path| path.into_os_string())
        .collect::<Vec<OsString>>();

    let mut out = Vec::<V4lCameraDevice>::new();
    for (i, dp) in device_paths.iter().enumerate() {
        out.push(match dp.clone().into_string() {
            Ok(str_path) => V4lCameraDevice::new(&str_path, i as u16),
            Err(os_path) => V4lCameraDevice::new(&os_path.to_string_lossy(), i as u16),
        })
    }

    Ok(out)
}

pub struct V4lCameraDevice {
    pub device_info: DeviceInformation,
    pub device: v4l::Device,
    pub camera_configs: Vec<CameraConfig>,
}

impl V4lCameraDevice {
    pub fn new(path: &str, id: u16) -> Self {
        let dev = v4l::Device::with_path(path).unwrap();
        // v4l::Device::new(id as usize).unwrap();

        let caps = dev.query_caps().unwrap();

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

        let device_info = DeviceInformation {
            id: id,
            name: caps.card,
            device_type: DeviceType::Camera,
        };

        V4lCameraDevice {
            device_info: device_info,
            device: dev,
            camera_configs: out_configs,
        }
    }
}
