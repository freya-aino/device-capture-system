use anyhow::Error;

use nokhwa::utils::{CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType};
use nokhwa::{native_api_backend, Camera};
use nokhwa::query;



// datamodel
pub trait Device {
    fn new(index: u16, name: String, device_string: Option<String>) -> Self;
    fn open(&self) -> Result<(), Error>;
    fn close(&self) -> Result<(), Error>;
    fn get_all_possible_configs<T>(&self) -> Result<Vec<T>, Error>;
    fn get_current_config<T>(&self) -> Result<Option<T>, Error>;
    fn set_config(&self, config: DeviceConfig) -> Result<(), Error>;
}

#[derive(Debug)]
enum DeviceState {
    Open,
    Closed,
    NoConfig,
    Error,
}

#[derive(Debug)]
pub struct CameraConfig {
    width: u16,
    height: u16,
    frame_rate: u16,
    pixel_format: FrameFormat,
}

#[derive(Debug)]
pub struct MicrophoneConfig {
    channels: u16,
    sample_rate: u32,
    sample_size: u16,
}

#[derive(Debug)]
struct DevicePrimitives {
    index: u16,
    name: String,
    device_string: Option<String>,
}

#[derive(Debug)]
pub struct CameraDevice {
    primitives: DevicePrimitives,
    config: Option<CameraConfig>,
}

#[derive(Debug)]
pub struct MicrophoneDevice {
    primitives: DevicePrimitives,
    config: Option<MicrophoneConfig>,
}



impl CameraConfig {
    pub fn new(width: u16, height: u16, frame_rate: u16, pixel_format: FrameFormat) -> Self {
        CameraConfig {
            width: width,
            height: height,
            frame_rate: frame_rate,
            pixel_format: pixel_format,
        }
    }
}


impl DevicePrimitives {
    pub fn new(index: u16, name: String, device_string: Option<String>) -> Self {
        DevicePrimitives { index, name, device_string }
    }
}

impl Device for CameraDevice {
    fn new(index: u16, name: String, device_string: Option<String>) -> Self {
        CameraDevice {
            primitives: DevicePrimitives::new(index, name, device_string),
            config: None,
        }
    }
    fn open(&self) -> Result<(), Error> {
        Ok(()) // todo
    }
    fn close(&self) -> Result<(), Error> {
        Ok(()) // todo
    }

    fn get_all_possible_configs(&self) -> Result<Vec<CameraConfig>, Error> {
        let index = CameraIndex::Index(self.primitives.index as u32);
        let format = RequestedFormat::<'_>::new(RequestedFormatType::AbsoluteHighestFrameRate);
        let camera = Camera::new(index, format).unwrap();
        let available_formats = camera.compatible_camera_formats().unwrap();

        let mut out_formats = Vec::new();
        for f in available_formats {

            out_formats.push(
                CameraConfig::new(
                    f.width() as u16, 
                    f.height() as u16,
                    f.frame_rate() as u16,
                    f.format(),
                )
            );
        }
        Ok(out_formats);
    }

    fn get_current_config(&self) -> Result<Option<CameraConfig>, Error> {
        Ok(self.config)
    }

    fn set_config(&self, config: CameraConfig) -> Result<(), Error> {
        self.config = Some(config);
        Ok(())
    }

}

impl Device for MicrophoneDevice {
    fn new(index: u16, name: String, device_string: Option<String>) -> Self {
        MicrophoneDevice {
            primitives: DevicePrimitives::new(index, name, device_string),
            config: Vec::new(),
        }
    }
    fn open(&self) -> Result<(), Error> {
        Ok(()) // todo
    }
    fn close(&self) -> Result<(), Error> {
        Ok(()) // todo
    }
}


// impl MicrophoneDevice {
//     pub fn new(index: u8, name: String, device_string: Option<String>, configs: Option<Vec<MicrophoneConfig>>) -> Self {
//         let configs = match configs {
//             Some(configs) => configs,
//             None => Vec::new(),
//         };
//         MicrophoneDevice {
//             index: index,
//             name: name,
//             device_string: device_string,
//             config: configs,
//         }
//     }
// }

// impl CameraDevice {
//     pub fn new(index: nokhwa::utils::CameraIndex, name: String, device_string: Option<String>, configs: Option<Vec<CameraConfig>>) -> Self {
//         let config = match configs {
//             Some(configs) => configs,
//             None => Vec::new(),
//         };
//         CameraDevice {
//             index: index,
//             name: name,
//             device_string: device_string,
//             config: config,
//         }
//     }
// }


// impl Device for CameraDevice {

// }

// impl Device for MicrophoneDevice {

// }