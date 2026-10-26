use anyhow::{Error, Result};
use bincode::{Decode, Encode, config};
use std::time::SystemTime;

use super::DeviceInformation;

#[derive(Debug, PartialEq, Clone)]
pub enum ConnectionStatus {
    Available,
    Active,
    Paused,
    Error,
    Closed,
}

#[derive(Debug, PartialEq, Encode, Decode, Clone)]
pub struct ConnectionStats {
    frames: u64,
    bytes: u64,
    current_fps: f32,
    current_bitrate: f32,
    current_latency: f32,
    start_time: SystemTime,
    last_frame_time: SystemTime,
}

impl ConnectionStats {
    pub fn new() -> Self {
        ConnectionStats {
            frames: 0,
            bytes: 0,
            current_fps: 0.0,
            current_bitrate: 0.0,
            current_latency: 0.0,
            start_time: SystemTime::now(),
            last_frame_time: SystemTime::now(),
        }
    }

    pub fn update(&mut self, bytes: u64) {
        self.frames += 1;
        self.bytes += bytes;
        self.current_fps = 1.0 / (self.last_frame_time.elapsed().unwrap().as_secs() as f32);
        self.current_bitrate =
            (self.bytes as f32) / (self.last_frame_time.elapsed().unwrap().as_secs_f32() * 1024.0);
        self.current_latency = self.last_frame_time.elapsed().unwrap().as_secs_f32();
        self.last_frame_time = SystemTime::now();
    }
}

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
pub struct FramePacketInformation {
    pub device_info: DeviceInformation,
    pub rx_timestamp: Option<SystemTime>,
    pub tx_timestamp: Option<SystemTime>,
    pub frame_shape: Vec<u32>,
}

impl FramePacketInformation {
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        let ser_info = bincode::encode_to_vec(&self, config::standard())
            .map_err(|e| Error::msg(format!("Serialization error: {}", e)))?;
        Ok(ser_info)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, Error> {
        let (info, _) = bincode::decode_from_slice(data, config::standard())
            .map_err(|e| Error::msg(format!("Deserialization error: {}", e)))?;
        Ok(info)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FramePacket {
    pub frame_info: FramePacketInformation,
    pub data: Box<[u8]>,
}

impl FramePacket {
    pub fn new(frame_info: FramePacketInformation, data: Box<[u8]>) -> Self {
        FramePacket { frame_info, data }
    }

    pub fn decode_to_i16_le(bytes: &[u8]) -> Result<Vec<i16>, Error> {
        assert!(
            bytes.len() % 2 == 0,
            "bytes length needs to be even for I16LE decoding"
        );
        let i16_samples = bytes
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<i16>>();
        Ok(i16_samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceStatus, DeviceType};

    fn dummy_device_info() -> DeviceInformation {
        DeviceInformation {
            id: "id".to_string(),
            name: "name".to_string(),
            device_type: DeviceType::Camera,
            device_status: DeviceStatus::Available,
        }
    }

    #[test]
    fn frame_packet_information_roundtrip() {
        let info = FramePacketInformation {
            device_info: dummy_device_info(),
            rx_timestamp: None,
            tx_timestamp: Some(SystemTime::now()),
            frame_shape: vec![1920, 1080],
        };

        let bytes = info.serialize().unwrap();
        let decoded = FramePacketInformation::deserialize(&bytes).unwrap();

        assert_eq!(info, decoded);
    }

    #[test]
    fn frame_packet_information_deserialize_invalid_data_errs() {
        let result = FramePacketInformation::deserialize(&[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_to_i16_le_round_trips_known_values() {
        let samples: Vec<i16> = vec![0, 1, -1, i16::MAX, i16::MIN];
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();

        let decoded = FramePacket::decode_to_i16_le(&bytes).unwrap();

        assert_eq!(samples, decoded);
    }

    #[test]
    #[should_panic(expected = "bytes length needs to be even")]
    fn decode_to_i16_le_odd_length_panics() {
        let _ = FramePacket::decode_to_i16_le(&[0u8, 1, 2]);
    }

    #[test]
    fn connection_stats_update_tracks_frames_and_bytes() {
        let mut stats = ConnectionStats::new();
        stats.update(100);

        assert_eq!(stats.frames, 1);
        assert_eq!(stats.bytes, 100);

        stats.update(50);
        assert_eq!(stats.frames, 2);
        assert_eq!(stats.bytes, 150);
    }
}
