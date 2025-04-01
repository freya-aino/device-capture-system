// use std::ops::Deref;
// use std::sync::Arc;
// use std::time::SystemTime;

// use anyhow::Error;
// use either::{Either, IntoEither};

// use cpal::traits::{DeviceTrait, HostTrait};
// use nokhwa::pixel_format::RgbFormat;
// use nokhwa::query;
// use nokhwa::utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType};
// use nokhwa::{Camera, native_api_backend, nokhwa_check};

// ---------------- ENUMS ----------------


// #[derive(Debug)]
// enum DeviceConfig {
//     Camera(CameraConfig),
//     Microphone(MicrophoneConfig),
// }

// ---------------- STRUCTS ----------------

// #[derive(Debug)]
// pub struct CaptureStats {
//     frames_captured: u64,
//     frames_droped: u64,
//     bytes_captured: u64,
//     current_fps: f32,
//     current_bitrate: f32,
//     current_latency: f32,
//     uptime_seconds: u64,
//     last_frame_time: SystemTime,
// }

// #[derive(Debug)]
// pub struct CaptureProcess {
//     device: Device,
//     process_id: u32,
//     port: u32,
//     start_time: SystemTime,
//     stats: CaptureStats,
// }

// ---------------- TRAITS ----------------
// trait Process {
//     fn initialize(&self) -> Result<(), Error>;
//     fn start(&self) -> Result<(), Error>;
//     fn stop(&self) -> Result<(), Error>;
//     fn pause(&self) -> Result<(), Error>;
//     fn resume(&self) -> Result<(), Error>;
//     fn restart(&self) -> Result<(), Error>;
//     fn get_stats(&self) -> CaptureStats;
// }

// ---------------- IMPLS ----------------

// // impl CaptureProcess {
// //     pub fn new(device_info: DeviceInfo, process_id: u32, port: u32, config: CaptureConfig) -> Self {
// //         CaptureProcess {
// //             device_info: device_info,
// //             process_id: process_id,
// //             port: port,
// //             start_time: SystemTime::now(),
// //             config: config,
// //             stats: CaptureStats {
// //                 frames_captured: 0,
// //                 frames_droped: 0,
// //                 bytes_captured: 0,
// //                 current_fps: 0.0,
// //                 current_bitrate: 0.0,
// //                 current_latency: 0.0,
// //                 uptime_seconds: 0,
// //                 last_frame_time: SystemTime::now(),
// //             },
// //         }
// //     }
// // }


// impl CaptureStats {
//     pub fn new() -> Self {
//         CaptureStats {
//             frames_captured: 0,
//             frames_droped: 0,
//             bytes_captured: 0,
//             current_fps: 0.0,
//             current_bitrate: 0.0,
//             current_latency: 0.0,
//             uptime_seconds: 0,
//             last_frame_time: SystemTime::now(),
//         }
//     }
// }

