use anyhow::{Error, Result};
use std::net::{Ipv4Addr, SocketAddrV4};
use zmq::{Context, Socket};

use shared::{ConnectionStats, ConnectionStatus};

// #[derive(Clone)]
pub struct Connection {
    pub address: SocketAddrV4,
    pub endpoint: String,
    pub queue_size: u32,
    pub data_chunk_size: u32,
    pub socket: Option<Socket>,
    pub socket_type: Option<zmq::SocketType>,
    pub status: ConnectionStatus,
    pub stats: ConnectionStats,
}

impl Connection {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32, data_chunk_size: u32) -> Self {
        let address = SocketAddrV4::new(host_address, port);
        let endpoint = format!("tcp://{}:{}", address.ip(), address.port());
        Connection {
            address,
            endpoint,
            queue_size: queue_size,
            data_chunk_size: data_chunk_size,
            socket: None,
            socket_type: None,
            status: ConnectionStatus::Available,
            stats: ConnectionStats::new(),
        }
    }

    pub fn new_endpoint(
        endpoint: impl Into<String>,
        queue_size: u32,
        data_chunk_size: u32,
    ) -> Self {
        Connection {
            address: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0),
            endpoint: endpoint.into(),
            queue_size,
            data_chunk_size,
            socket: None,
            socket_type: None,
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
        let socket = context.socket(socket_type)?;
        socket.set_linger(0)?;
        socket.set_sndhwm(self.queue_size as i32)?;
        socket.set_rcvhwm(self.queue_size as i32)?;

        match socket_type {
            zmq::PUB | zmq::XPUB => {
                socket.bind(&self.endpoint)?;
            }
            zmq::SUB => {
                socket.connect(&self.endpoint)?;
                socket.set_subscribe(&[])?; // subscribe to all topics
            }
            zmq::XSUB => {
                // ZMQ_SUBSCRIBE is only valid on SUB sockets. XSUB sockets forward
                // subscription messages they receive on their paired XPUB socket
                // instead (handled automatically by zmq::proxy/proxy_steerable).
                socket.connect(&self.endpoint)?;
            }
            _ => {
                return Err(Error::msg("Unsupported socket type."));
            }
        }
        self.socket = Some(socket);
        self.socket_type = Some(socket_type);
        self.stats = ConnectionStats::new();
        self.status = ConnectionStatus::Active;
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), Error> {
        let soc = match self.socket.take() {
            Some(soc) => soc,
            None => {
                println!("Socket is already closed.");
                return Ok(());
            }
        };

        match self.socket_type.take() {
            Some(zmq::PUB) | Some(zmq::XPUB) => {
                soc.unbind(&self.endpoint)?;
            }
            Some(zmq::SUB) | Some(zmq::XSUB) => {
                soc.disconnect(&self.endpoint)?;
            }
            _ => {}
        }

        self.socket = None;
        self.status = ConnectionStatus::Closed;
        Ok(())
    }

    pub fn update_stats(&mut self, num_data_bytes: u64) {
        self.stats.update(num_data_bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ENDPOINT_ID: AtomicUsize = AtomicUsize::new(0);

    fn new_connection() -> Connection {
        let id = ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
        Connection::new_endpoint(format!("inproc://connection-test-{}", id), 10, 1024)
    }

    #[test]
    fn new_connection_starts_available_without_socket() {
        let connection = new_connection();
        assert_eq!(connection.status, ConnectionStatus::Available);
        assert!(connection.socket.is_none());
        assert!(connection.get_socket().is_err());
    }

    #[test]
    fn open_rejects_unsupported_socket_type() {
        let context = Context::new();
        let mut connection = new_connection();

        let result = connection.open(&context, zmq::REQ);

        assert!(result.is_err());
        assert!(connection.socket.is_none());
    }

    #[test]
    fn open_and_close_pub_socket_sets_status() {
        let context = Context::new();
        let mut connection = new_connection();

        connection.open(&context, zmq::PUB).unwrap();
        assert_eq!(connection.status, ConnectionStatus::Active);
        assert!(connection.get_socket().is_ok());

        connection.close().unwrap();
        assert_eq!(connection.status, ConnectionStatus::Closed);
        assert!(connection.socket.is_none());
    }

    #[test]
    fn close_without_open_is_a_no_op() {
        let mut connection = new_connection();
        assert!(connection.close().is_ok());
        assert_eq!(connection.status, ConnectionStatus::Available);
    }
}
