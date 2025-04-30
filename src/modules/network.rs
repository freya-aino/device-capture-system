use anyhow::{Error, Result};
use bincode::config;
use bincode::{Decode, Encode};

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::SystemTime;

use zmq::{Context, Socket};

// use crate::DeviceInformation;

#[derive(Debug, PartialEq)]
pub enum ConnectionStatus {
    Created,
    Initialized,
    Active,
    Paused,
    Error,
    Closed,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct FramePacketInformation {
    timestamp: SystemTime,
    frame_shape: Vec<u16>,
}

impl FramePacketInformation {
    pub fn new(timestamp: SystemTime, frame_shape: Vec<u16>) -> Self {
        FramePacketInformation {
            timestamp,
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

#[derive(Debug)]
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

pub struct Connection {
    address: SocketAddrV4,
    queue_size: u32,
    socket: Option<Socket>,
    status: ConnectionStatus,
    pub stats: Option<ConnectionStats>,
}

pub struct Sender(Connection);
pub struct Receiver(Connection);

pub struct Proxy {
    from: Connection,
    to: Connection,
}

impl Receiver {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32) -> Self {
        Receiver(Connection::new(host_address, port, queue_size))
    }

    pub fn initialize(&mut self, context: &Context) -> Result<(), Error> {
        self.0.initialize(context, zmq::SUB)
    }

    pub fn receive(&mut self, zmq_flags: i32) -> Result<Option<FramePacket>, Error> {
        assert!(
            self.0.status != ConnectionStatus::Closed,
            "Trying to receive while connection is closed."
        );
        assert!(
            self.0.status != ConnectionStatus::Created,
            "Trying to receive while connection is not initialized."
        );
        assert!(
            !self.0.socket.is_none(),
            "Trying to receive while socket is None (Status is not correct, this might hint at a previous function having exited unexpetedly)."
        );

        if self.0.status == ConnectionStatus::Paused {
            println!("Calling receive while connection is paused...");
            return Ok(None);
        }

        let socket = self.0.get_socket()?;

        match socket.recv_multipart(zmq_flags) {
            Ok(parts) => {
                assert!(parts.len() >= 2, "Expected 2 or more parts in the message.");

                let rcv_ts = SystemTime::now();

                let frame_information = FramePacketInformation::deserialize(&parts[0]).unwrap();
                let data = parts[1..].concat().to_vec();

                self.0.set_status(ConnectionStatus::Active);
                self.0.update_stats(data.len() as u64);

                return Ok(Some(FramePacket::new(
                    rcv_ts,
                    frame_information.frame_shape,
                    data,
                )));
            }
            Err(e) if e == zmq::Error::EAGAIN => {
                // No message available, continue waiting
                return Ok(None);
            }
            Err(e) => {
                self.0.set_status(ConnectionStatus::Error);
                return Err(Error::msg(format!("Error receiving message: {}", e)));
            }
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

    pub fn send(
        &mut self,
        frame_packet: FramePacket,
        zmq_flags: i32,
        data_chunk_size: u32,
    ) -> Result<(), Error> {
        assert!(
            data_chunk_size > 0,
            "Data chunk size must be greater than 0."
        );

        assert!(
            self.0.status != ConnectionStatus::Closed,
            "Trying to send while connection is closed."
        );
        assert!(
            self.0.status != ConnectionStatus::Created,
            "Trying to send while connection is not initialized."
        );
        assert!(
            !self.0.socket.is_none(),
            "Trying to send while socket is None (Status is not correct, this might hint at a previous function having exited unexpetedly)."
        );

        if self.0.status == ConnectionStatus::Paused {
            println!("Calling send while connection is paused...");
            return Ok(());
        }

        let socket = self.0.get_socket()?;
        let serialized_info = frame_packet.frame_info.serialize()?;
        let data = frame_packet.data.as_ref();

        let mut chunked_message: Vec<&[u8]> = Vec::new();

        // append data to chunked message
        chunked_message.push(&serialized_info);
        for chunk in data.chunks(data_chunk_size as usize) {
            chunked_message.push(chunk);
        }

        socket.send_multipart(&chunked_message, zmq_flags)?;

        self.0.set_status(ConnectionStatus::Active);
        self.0
            .update_stats((serialized_info.len() + frame_packet.data.len()) as u64);
        return Ok(());
    }
}

impl Proxy {
    pub fn new(
        from_ip: Ipv4Addr,
        from_port: u16,
        to_ip: Ipv4Addr,
        to_port: u16,
        queue_size: u32,
    ) -> Self {
        Proxy {
            from: Connection::new(from_ip, from_port, queue_size),
            to: Connection::new(to_ip, to_port, queue_size),
        }
    }

    pub fn initialize(&mut self, context: &Context) -> Result<(), Error> {
        self.from.initialize(context, zmq::XSUB)?;
        self.to.initialize(context, zmq::XPUB)?;
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

    pub fn get_socket(&self) -> Result<&Socket, Error> {
        match self.socket {
            Some(ref socket) => Ok(socket),
            None => Err(Error::msg("Socket is not initialized.")),
        }
    }

    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status;
    }

    pub fn initialize(
        &mut self,
        context: &Context,
        socket_type: zmq::SocketType,
    ) -> Result<(), Error> {
        let endpoint = format!("tcp://{}:{}", self.address.ip(), self.address.port());

        let socket = context.socket(socket_type)?;
        socket.set_linger(0)?;
        socket.set_sndhwm(self.queue_size as i32)?;
        socket.set_rcvhwm(self.queue_size as i32)?;

        match socket_type {
            zmq::PUB | zmq::XPUB => {
                socket.bind(&endpoint)?;
            }
            zmq::SUB | zmq::XSUB => {
                socket.connect(&endpoint)?;
                socket.set_subscribe(&[])?; // subscribe to all topics
            }
            _ => {
                return Err(Error::msg("Unsupported socket type."));
            }
        }
        self.socket = Some(socket);
        self.stats = Some(ConnectionStats::new());
        self.set_status(ConnectionStatus::Initialized);
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

    pub fn update_stats(&mut self, bytes: u64) {
        if let Some(ref mut stats) = self.stats {
            stats.update(bytes);
        }
    }
}
