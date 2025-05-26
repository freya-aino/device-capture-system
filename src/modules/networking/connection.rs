use anyhow::{Error, Result};

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::SystemTime;

use zmq::{Context, Socket};


pub struct Connection {
    address: SocketAddrV4,
    queue_size: u32,
    socket: Option<Socket>,
    status: ConnectionStatus,
    stats: Option<ConnectionStats>,
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
