// use anyhow::{Error, Result};
// use bincode::{Decode, Encode, config};
// use std::time::SystemTime;


// #[derive(Debug, PartialEq)]
// pub enum ConnectionStatus {
//     Created,
//     Initialized,
//     Active,
//     Paused,
//     Error,
//     Closed,
// }

// #[derive(Debug, PartialEq, Encode, Decode)]
// pub struct ConnectionStats {
//     frames: u64,
//     bytes: u64,
//     current_fps: f32,
//     current_bitrate: f32,
//     current_latency: f32,
//     start_time: SystemTime,
//     last_frame_time: SystemTime,
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

// impl ConnectionStats {
//     pub fn new() -> Self {
//         ConnectionStats {
//             frames: 0,
//             bytes: 0,
//             current_fps: 0.0,
//             current_bitrate: 0.0,
//             current_latency: 0.0,
//             start_time: SystemTime::now(),
//             last_frame_time: SystemTime::now(),
//         }
//     }

//     pub fn update(&mut self, bytes: u64) {
//         self.frames += 1;
//         self.bytes += bytes;
//         self.current_fps = 1.0 / (self.last_frame_time.elapsed().unwrap().as_secs() as f32);
//         self.current_bitrate =
//             (self.bytes as f32) / (self.last_frame_time.elapsed().unwrap().as_secs_f32() * 1024.0);
//         self.current_latency = self.last_frame_time.elapsed().unwrap().as_secs_f32();
//         self.last_frame_time = SystemTime::now();
//     }
// }

// #[derive(Debug, Clone, Encode, Decode)]
// pub struct FramePacketInformation {
//     pub device_info: DeviceInformation,
//     pub rx_timestamp: Option<SystemTime>,
//     pub tx_timestamp: Option<SystemTime>,
//     pub frame_shape: Vec<u16>,
// }

// impl FramePacketInformation {
//     pub fn serialize(&self) -> Result<Vec<u8>, Error> {
//         let ser_info = bincode::encode_to_vec(&self, config::standard())
//             .map_err(|e| Error::msg(format!("Serialization error: {}", e)))?;
//         Ok(ser_info)
//     }

//     pub fn deserialize(data: &[u8]) -> Result<Self, Error> {
//         let (info, _) = bincode::decode_from_slice(data, config::standard())
//             .map_err(|e| Error::msg(format!("Deserialization error: {}", e)))?;
//         Ok(info)
//     }
// }

// #[derive(Debug)]
// pub struct FramePacket {
//     pub frame_info: FramePacketInformation,
//     pub data: Box<[u8]>,
// }

// impl FramePacket {
//     pub fn new(frame_info: FramePacketInformation, data: Box<[u8]>) -> Self {
//         FramePacket { frame_info, data }
//     }
// }
