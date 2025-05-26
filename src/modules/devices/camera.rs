use v4l::FourCC;
use v4l::frameinterval::FrameIntervalEnum;
use v4l::video::Capture;


pub struct V4lCameraDevice {
    device_info: DeviceInformation,
    device: v4l::Device,
    camera_configs: Vec<CameraConfig>,
}

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
            DeviceInformation {
                id: id,
                name: caps.card,
                device_type: DeviceType::Camera,
            },
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
