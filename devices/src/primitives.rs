// use anyhow::Error;
// use bincode::{Decode, Encode};
// use glob::glob;
// use std::ffi::OsString;

// fn all_devices_paths_linux() -> Result<Vec<OsString>, Error> {
//     let device_paths = glob("/dev/video*")?
//         .filter_map(Result::ok)
//         .map(|path| path.into_os_string())
//         .collect::<Vec<OsString>>();
//     Ok(device_paths)
// }

// #[derive(Debug, Encode, Decode, Clone)]
// pub enum DeviceType {
//     Camera,
//     Microphone,
// }

// #[derive(Debug, Encode, Decode, Clone)]
// pub struct DeviceInformation {
//     pub id: u16,
//     pub name: String,
//     pub device_type: DeviceType,
// }

// #[derive(Debug)]
// pub struct CameraConfig {
//     pub width: u32,
//     pub height: u32,
//     pub fps: f32,
//     pub fourcc: String,
// }

// #[derive(Debug)]
// pub struct MicrophoneConfig {
//     pub sample_rate: u32,
//     pub channels: u8,
//     pub sample_size: u16,
// }
