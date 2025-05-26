// use bincode::Encode;
// use glob::glob;
// use std::ffi::OsString;
// use std::time::Duration;

// use anyhow::Ok;
// use anyhow::{Error, Result};

// use device_capture_system::{DeviceInformation, DeviceType};

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

// pub enum Device {
//     Microphone(CpalMicrophoneDevice),
//     Camera(V4lCameraDevice),
// }

// pub struct DeviceManager {
//     pub system_id: u8,
//     pub device_info: Device,
// }

// impl DeviceManager {
//     pub fn get_all_available_cameras() -> Result<Vec<V4lCameraDevice>, Error> {
//         let device_paths = all_devices_paths_linux().unwrap();
//         println!("Found {} devices!", device_paths.len());
//         if device_paths.len() == 0 {
//             println!("No devices found!");
//             return Ok(vec![]);
//         }
//         Ok(vec![])
//         // let mut cam = CameraDevice::new(device_paths[0].clone(), device_paths[0].clone());
//         // let confs = cam.get_all_available_configs()
//         //     .unwrap()
//         //     .iter()
//         //     .map(|config| DeviceConfig::Camera(config.clone()))
//         //     .collect::<Vec<DeviceConfig>>();
//         // Ok(confs)
//     }
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
