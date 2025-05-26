
// network.rs
#[derive(Debug, PartialEq)]
pub enum ConnectionStatus {
    Created,
    Initialized,
    Active,
    Paused,
    Error,
    Closed,
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


#[derive(Debug)]
pub struct FramePacket {
    pub frame_info: FramePacketInformation,
    pub data: Box<[u8]>,
}

impl FramePacket {
    pub fn new(timestamp: SystemTime, frame_shape: Vec<u16>, data: Vec<u8>) -> Self {
        FramePacket {
            frame_info: FramePacketInformation::new(timestamp, frame_shape),
            data: data.into_boxed_slice(),
        }
    }
}


#[derive(Debug, Clone, Encode, Decode)]
pub struct FramePacketInformation {
    timestamp: SystemTime,
    device_info: DeviceInformation,
    frame_shape: Vec<u16>,
}

impl FramePacketInformation {
    pub fn new(timestamp: SystemTime, device_info: DeviceInformation, frame_shape: Vec<u16>) -> Self {
        FramePacketInformation {
            timestamp,
            device_info,
            frame_shape,
        }
    }

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
