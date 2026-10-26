use std::net::Ipv4Addr;
use std::time::SystemTime;

use anyhow::{Error, Result};
use zmq::Error::EAGAIN;
use zmq::{Context, SUB};

use super::Connection;
use shared::{ConnectionStatus, FramePacket, FramePacketInformation};

pub struct Receiver(Connection);

impl Receiver {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32, data_chunk_size: u32) -> Self {
        Receiver(Connection::new(
            host_address,
            port,
            queue_size,
            data_chunk_size,
        ))
    }

    pub fn new_endpoint(
        endpoint: impl Into<String>,
        queue_size: u32,
        data_chunk_size: u32,
    ) -> Self {
        Receiver(Connection::new_endpoint(
            endpoint,
            queue_size,
            data_chunk_size,
        ))
    }

    pub fn connection_status(&self) -> &ConnectionStatus {
        &self.0.status
    }

    pub fn data_chunk_size(&self) -> u32 {
        self.0.data_chunk_size
    }

    pub fn start(&mut self, context: &Context) -> Result<(), Error> {
        self.0.open(context, SUB)
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.0.close()
    }

    pub fn receive(&mut self, zmq_flags: i32) -> Result<Option<FramePacket>, Error> {
        assert!(
            self.0.status != ConnectionStatus::Closed,
            "Trying to receive while connection is closed."
        );
        assert!(
            !self.0.socket.is_none(),
            "Trying to receive while socket is None (Status is not correct, this might hint at a previous function having exited unexpectedly)."
        );

        if self.0.status == ConnectionStatus::Paused {
            return Ok(None);
        }

        let socket = self.0.get_socket()?;

        match socket.recv_multipart(zmq_flags) {
            Ok(parts) => {
                if parts.len() < 2 {
                    eprintln!(
                        "Dropping malformed frame: expected 2 or more parts, got {}.",
                        parts.len()
                    );
                    return Ok(None);
                }

                let mut frame_information = match FramePacketInformation::deserialize(&parts[0]) {
                    Ok(info) => info,
                    Err(e) => {
                        eprintln!("Dropping frame with undecodable header: {}", e);
                        return Ok(None);
                    }
                };
                let tx_timestamp = match frame_information.tx_timestamp {
                    Some(tx) => tx,
                    None => {
                        eprintln!("Dropping frame without tx_timestamp.");
                        return Ok(None);
                    }
                };

                let rx_timestamp = SystemTime::now();
                frame_information.rx_timestamp = Some(rx_timestamp);
                let data = parts[1..].concat().into_boxed_slice();

                self.0.status = ConnectionStatus::Active;
                self.0.update_stats(data.len() as u64);
                if let Ok(latency) = rx_timestamp.duration_since(tx_timestamp) {
                    self.0.stats.record_latency(latency);
                }

                return Ok(Some(FramePacket::new(frame_information, data)));
            }
            Err(e) if e == EAGAIN => {
                // No message available, continue waiting
                return Ok(None);
            }
            Err(e) => {
                self.0.status = ConnectionStatus::Error;
                return Err(Error::msg(format!("Error receiving message: {}", e)));
            }
        }
    }
}
