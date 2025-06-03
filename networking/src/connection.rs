use anyhow::{Error, Result};
use std::net::{Ipv4Addr, SocketAddrV4};
use zmq::{Context, Socket};

use shared::{ConnectionStats, ConnectionStatus};

// #[derive(Clone)]
pub struct Connection {
    pub address: SocketAddrV4,
    pub queue_size: u32,
    pub data_chunk_size: u32,
    pub socket: Option<Socket>,
    pub status: ConnectionStatus,
    pub stats: ConnectionStats,
}

impl Connection {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32, data_chunk_size: u32) -> Self {
        Connection {
            address: SocketAddrV4::new(host_address, port),
            queue_size: queue_size,
            data_chunk_size: data_chunk_size,
            socket: None,
            status: ConnectionStatus::Available,
            stats: ConnectionStats::new(),
        }
    }

    pub fn connection_status(&self) -> &ConnectionStatus {
        &self.status
    }

    pub fn get_socket(&self) -> Result<&Socket, Error> {
        match self.socket {
            Some(ref socket) => Ok(socket),
            None => Err(Error::msg("Socket is not initialized.")),
        }
    }

    pub fn open(&mut self, context: &Context, socket_type: zmq::SocketType) -> Result<(), Error> {
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
        self.stats = ConnectionStats::new();
        self.status = ConnectionStatus::Active;
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
        Ok(())
    }

    pub fn update_stats(&mut self, num_data_bytes: u64) {
        self.stats.update(num_data_bytes);
    }
}
