use anyhow::{Error, Result};
use bincode::config::Configuration;
use portpicker::pick_unused_port;
use serde::{Deserialize, Serialize};
use bincode::config;
use bincode::{Encode, Decode};

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::{self, Duration, SystemTime};

use zmq::{Context, Socket};

use crate::DeviceInformation;



pub enum ConnectionStatus {
    Created,
    Initialized,
    Active,
    Paused,
    Closed,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct FramePacketInformation {
    timestamp: time::SystemTime,
    device_information: DeviceInformation,
    frame_shape: Vec<u16>,
}

impl FramePacketInformation {
    pub fn new(
        timestamp: time::SystemTime,
        device_information: DeviceInformation,
        frame_shape: Vec<u16>,
    ) -> Self {
        FramePacketInformation {
            timestamp,
            device_information,
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


#[derive(Debug)]
pub struct FramePacket<'a> {
    frame_info: FramePacketInformation,
    data: &'a [u8],
}

impl<'a> FramePacket<'a> {
    pub fn new(
        timestamp: time::SystemTime,
        device_information: DeviceInformation,
        frame_shape: Vec<u16>,
        data: &'a [u8],
    ) -> Self {
        FramePacket {
            frame_info: FramePacketInformation::new(
                timestamp,
                device_information,
                frame_shape,
            ),
            data: data,
        }
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
}

impl Sender {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32) -> Self {
        Sender(Connection::new(host_address, port, queue_size))
    }

    pub fn initialize(&mut self, context: &Context) -> Result<(), Error> {
        self.0.initialize(context, zmq::PUB)
    }
    
    pub fn send<'a>(&self, frame_packet: FramePacket<'a>) -> Result<(), Error> {
        match self.0.socket {
            Some(ref socket) => {
                let serialized_info = frame_packet.frame_info.serialize()?;
                socket.send_multipart(&[
                    serialized_info.as_slice(),
                    frame_packet.data,
                ], 0)?;
            },
            None => {
                return Err(Error::msg("Socket is not initialized."));
            }
        };
        Ok(())
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

