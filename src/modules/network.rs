use anyhow::{Error, Result};
use portpicker::pick_unused_port;

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use zmq::{Context, Socket};

use crate::DeviceInformation;


// fn generate_random_image(w: u32, h: u32, c: u32) -> Vec<u8> {
//     let size = (w * h * c) as usize;
//     let arr = vec![0u8; size];
//     arr
// }

pub enum ConnectionStatus {
    Created,
    Initialized,
    Active,
    Paused,
    Closed,
}

#[derive(Debug)]
pub struct FramePacket {
    timestamp: u64,
    device_information: DeviceInformation,
    frame_shape: Vec<u16>,
    frame: Vec<u8>,
}

impl FramePacket {
    pub fn new(
        timestamp: u64,
        device_information: DeviceInformation,
        frame_shape: Vec<u16>,
        frame: Vec<u8>,
    ) -> Self {
        FramePacket {
            timestamp,
            device_information,
            frame_shape,
            frame,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        let mut serialized_data = Vec::new();
        serialized_data.extend_from_slice(&self.timestamp.to_le_bytes());
        serialized_data.extend_from_slice(&self.device_information.serialize()?);
        serialized_data.extend_from_slice(&self.frame_shape.serialize()?);
        serialized_data.extend_from_slice(&self.frame);
        Ok(serialized_data)
    }
}



#[derive(Debug)]
pub struct ConnectionStats {
    frames: u64,
    bytes: u64,
    current_fps: f32,
    current_bitrate: f32,
    current_latency: f32,
    uptime_seconds: u64,
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
            uptime_seconds: 0,
            last_frame_time: SystemTime::now(),
        }
    }
}


trait Connect {
    fn initialize(&self) -> Result<(), Error>;
    // fn start(&self) -> Result<(), Error>;
    // fn stop(&self) -> Result<(), Error>;
    // fn pause(&self) -> Result<(), Error>;
    // fn resume(&self) -> Result<(), Error>;
    // fn restart(&self) -> Result<(), Error>;
    // fn get_stats(&self) -> ConnectionStats;
}

pub struct Connection {
    address: SocketAddrV4,
    queue_size: u32,
    socket: Option<Socket>,
    status: ConnectionStatus,
    stats: Option<ConnectionStats>,
}

pub struct Sender(Connection);
pub struct Receiver(Connection);

// impl Connect for Sender {
    
// }

impl Receiver {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32) -> Self {
        Receiver(Connection::new(host_address, port, queue_size))
    }

    pub fn initialize(&mut self, context: &Context) -> Result<(), Error> {
        self.0.initialize(context, zmq::SUB)
    }

    pub fn send(&mut self, frame_packet: FramePacket) -> Result<(), Error> {
        if let Some(socket) = &self.0.socket {
            socket.send_multipart(vec![
                device_info,
                data.to_vec(),
            ]);
            Ok(())
        } else {
            Err(Error::msg("Socket is not initialized."))
        }
    }
}

impl Sender {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32) -> Self {
        Sender(Connection::new(host_address, port, queue_size))
    }

    pub fn initialize(&mut self, context: &Context) -> Result<(), Error> {
        self.0.initialize(context, zmq::PUB)
    }
}

impl Connection {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32) -> Self {
        Connection {
            address: SocketAddrV4::new(host_address, port),
            queue_size: queue_size,
            socket: None,
            status: ConnectionStatus::Created,
            stats: None,
        }
    }

    pub fn initialize(&mut self, context: &Context, socket_type: zmq::SocketType) -> Result<(), Error> {
        let socket = context.socket(socket_type)?;
        match socket_type {
            zmq::PUB => {
                socket.set_sndhwm(self.queue_size as i32)?;
            },
            zmq::SUB => {
                socket.set_rcvhwm(self.queue_size as i32)?;
            },
            _ => {
                return Err(Error::msg("Unsupported socket type."));
            }
        }

        socket.connect(&format!("tcp://{}:{}", self.address.ip(), self.address.port()))?;
        self.socket = Some(socket);
        self.stats = Some(ConnectionStats::new());
        self.status = ConnectionStatus::Initialized;
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), Error> {
        let soc = self.socket.take();

        if soc.is_none() {
            println!("Socket is already closed.");
            return Ok(());
        }

        let endpoint = format!("tcp://{}:{}", self.address.ip(), self.address.port());

        soc.unwrap().unbind(endpoint.as_str()).unwrap();

        self.socket = None;
        self.status = ConnectionStatus::Closed;
        self.stats = None;
        Ok(())
    }
}


// #[derive(Debug)]
// pub struct FramePacket {
//     frame: Vec<u8>,
//     device_information: DeviceInformation,
//     timestamp: u64,
//     frame_shape: Vec<u16>,
// }

// pub struct Connection {
//     ip: String,
//     port: String,
// }

// pub struct WebRTCManager {
//     peer_connection: Arc<RTCPeerConnection>,
//     data_channel: Option<Arc<RTCDataChannel>>,
// }





