use anyhow::Error;
use glob::glob;
use shared::{Device, DeviceConfig, DeviceStatus};
use std::{ffi::OsString, thread::JoinHandle};

use crate::{AlsaMicrophoneDevice, V4lCameraDevice};

// pub struct DeviceManager {
//     pub device_status: DeviceStatus,
//     pub handle: Option<JoinHandle<Result<(), Error>>>,
//     pub ip: Option<String>,
//     pub port: Option<u16>,
// }

// pub fn get_all_alsa_devices() -> Result<Vec<AlsaMicrophoneDevice, Error> {

//     let mut out_devices = Vec::<AlsaMicrophoneDevice>::new();

//     let hints = HintIter::new_str(None, "pcm")?;
//     for hint in hints {
//         if !hint.direction.is_none() || hint.direction == Some(Direction::Playback) {
//             continue;
//         }

//         out_device_infos.push(AlsaMicrophoneDevice::new(
//             // todo
//         ))
//     }

//     Ok(out_devices)
// }

// pub struct DeviceManager<D: Device> {
//     pub device: D,
//     pub device_status: DeviceStatus,
//     pub ip: Option<String>,
//     pub port: Option<u16>,
// }

// impl DeviceManager<AlsaMicrophoneDevice> {
//     pub fn new(
//         device: AlsaMicrophoneDevice,
//         device_status: DeviceStatus,
//         ip: Option<String>,
//         port: Option<u16>,
//     ) -> Self {
//         Self {
//             device: device,
//             device_status: device_status,
//             ip: ip,
//             port: port,
//         }
//     }

// }

// impl DeviceManager<CpalMicrophoneDevice> {
//     pub fn new(
//         device: CpalMicrophoneDevice,
//         device_status: DeviceStatus,
//         ip: Option<String>,
//         port: Option<u16>,
//     ) -> Self {
//         Self {
//             device: device,
//             device_status: device_status,
//             ip: ip,
//             port: port,
//         }
//     }

//     pub fn get_all_devices(cpal_host: &Host) -> Result<Vec<Self>, Error> {
//         // let host = cpal::default_host();
//         let cpal_devices = cpal_host.input_devices()?;

//         let mut devices = Vec::<CpalMicrophoneDevice>::new();
//         for (i, dev) in cpal_devices.enumerate() {
//             devices.push(CpalMicrophoneDevice::new(dev, "".to_string()));
//         }

//         let mut out = Vec::<Self>::new();
//         for device in devices {
//             out.push(Self::new(device, DeviceStatus::Available, None, None));
//         }

//         Ok(out)
//     }
// }

// impl DeviceManager<V4lCameraDevice> {
//     pub fn new(
//         device: V4lCameraDevice,
//         device_status: DeviceStatus,
//         ip: Option<String>,
//         port: Option<u16>,
//     ) -> Self {
//         Self {
//             device: device,
//             device_status: device_status,
//             ip: ip,
//             port: port,
//         }
//     }

//     pub fn get_all_devices() -> Result<Vec<Self>, Error> {
//         let device_paths = glob("/dev/video*")?
//             .filter_map(Result::ok)
//             .map(|path| path.into_os_string())
//             .collect::<Vec<OsString>>();

//         let mut devices = Vec::<V4lCameraDevice>::new();
//         for dp in device_paths.iter() {
//             devices.push(match dp.clone().into_string() {
//                 Ok(str_path) => V4lCameraDevice::new(str_path),
//                 Err(os_path) => V4lCameraDevice::new(os_path.to_string_lossy().to_string()),
//             })
//         }

//         let mut out = Vec::<Self>::new();
//         for device in devices {
//             out.push(Self::new(device, DeviceStatus::Available, None, None));
//         }

//         Ok(out)
//     }
// }

// impl DeviceManager<CpalMicrophoneDevice> {
//     pub fn new(
//         device: CpalMicrophoneDevice,
//         // rx: Receiver<DeviceCommand>,
//         device_status: DeviceStatus,
//         ip: Option<String>,
//         port: Option<u16>,
//     ) -> Self {
//         Self {
//             device: device,
//             device_status: device_status,
//             // rx: rx,
//             ip: ip,
//             port: port,
//         }
//     }

//     pub fn get_all_devices() -> Result<Vec<Self>, Error> {
//         let cpal_devices = cpal_host.input_devices()?;

//         let mut devices = Vec::<CpalMicrophoneDevice>::new();
//         for (i, dev) in cpal_devices.enumerate() {
//             devices.push(CpalMicrophoneDevice::new(dev, i as u16));
//         }

//         let mut out = Vec::<Self>::new();
//         for device in devices {
//             out.push(Self::new(device, DeviceStatus::Available, None, None));
//         }

//         Ok(out)
//     }
// }

// impl DeviceManager<V4lCameraDevice> {
//     pub fn get_all_devices() -> Result<Vec<Self>, Error> {
//         let device_names = V4lCameraDevice::get_all_available_devices()?;

//         let mut devices = Vec::<V4lCameraDevice>::new();
//         for (i, name) in device_names.iter().enumerate() {
//             devices.push(V4lCameraDevice::new(name, i as u16));
//         }

//         let mut out = Vec::<Self>::new();
//         for device in devices {
//             out.push(Self::new(device, DeviceStatus::Available, None, None));
//         }

//         Ok(out)
//     }
// }
