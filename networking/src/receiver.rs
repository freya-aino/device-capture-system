use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::SystemTime;

use anyhow::{Error, Result};
use zmq::Error::EAGAIN;
use zmq::{Context, SUB};

use super::Connection;
use shared::{ConnectionStatus, FramePacket, FramePacketInformation};

pub struct Receiver {
    connection: Connection,
    last_sequence: HashMap<String, u64>,
    dropped_frames: u64,
}

impl Receiver {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32, data_chunk_size: u32) -> Self {
        Receiver {
            connection: Connection::new(host_address, port, queue_size, data_chunk_size),
            last_sequence: HashMap::new(),
            dropped_frames: 0,
        }
    }

    pub fn new_endpoint(
        endpoint: impl Into<String>,
        queue_size: u32,
        data_chunk_size: u32,
    ) -> Self {
        Receiver {
            connection: Connection::new_endpoint(endpoint, queue_size, data_chunk_size),
            last_sequence: HashMap::new(),
            dropped_frames: 0,
        }
    }

    pub fn connection_status(&self) -> &ConnectionStatus {
        &self.connection.status
    }

    pub fn data_chunk_size(&self) -> u32 {
        self.connection.data_chunk_size
    }

    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }

    pub fn start(&mut self, context: &Context) -> Result<(), Error> {
        self.connection.open(context, SUB)
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.connection.close()
    }

    fn record_sequence(&mut self, device_id: &str, sequence: u64) {
        if let Some(&last) = self.last_sequence.get(device_id) {
            if sequence > last + 1 {
                self.dropped_frames += sequence - last - 1;
            }
        }
        self.last_sequence.insert(device_id.to_string(), sequence);
    }

    pub fn receive(&mut self, zmq_flags: i32) -> Result<Option<FramePacket>, Error> {
        assert!(
            self.connection.status != ConnectionStatus::Closed,
            "Trying to receive while connection is closed."
        );
        assert!(
            !self.connection.socket.is_none(),
            "Trying to receive while socket is None (Status is not correct, this might hint at a previous function having exited unexpectedly)."
        );

        if self.connection.status == ConnectionStatus::Paused {
            return Ok(None);
        }

        let socket = self.connection.get_socket()?;

        match socket.recv_multipart(zmq_flags) {
            Ok(parts) => {
                if parts.len() < 3 {
                    tracing::warn!(
                        "Dropping malformed frame: expected 3 or more parts, got {}.",
                        parts.len()
                    );
                    return Ok(None);
                }

                let sequence = match <[u8; 8]>::try_from(parts[1].as_slice()) {
                    Ok(bytes) => u64::from_le_bytes(bytes),
                    Err(_) => {
                        tracing::warn!("Dropping frame with malformed sequence number.");
                        return Ok(None);
                    }
                };

                let mut frame_information = match FramePacketInformation::deserialize(&parts[0]) {
                    Ok(info) => info,
                    Err(e) => {
                        tracing::warn!("Dropping frame with undecodable header: {}", e);
                        return Ok(None);
                    }
                };
                let tx_timestamp = match frame_information.tx_timestamp {
                    Some(tx) => tx,
                    None => {
                        tracing::warn!("Dropping frame without tx_timestamp.");
                        return Ok(None);
                    }
                };

                self.record_sequence(&frame_information.device_info.id, sequence);

                let rx_timestamp = SystemTime::now();
                frame_information.rx_timestamp = Some(rx_timestamp);
                let data = parts[2..].concat().into_boxed_slice();

                self.connection.status = ConnectionStatus::Active;
                self.connection.update_stats(data.len() as u64);
                if let Ok(latency) = rx_timestamp.duration_since(tx_timestamp) {
                    self.connection.stats.record_latency(latency);
                }

                return Ok(Some(FramePacket::new(frame_information, data)));
            }
            Err(e) if e == EAGAIN => {
                // No message available, continue waiting
                return Ok(None);
            }
            Err(e) => {
                self.connection.status = ConnectionStatus::Error;
                return Err(Error::msg(format!("Error receiving message: {}", e)));
            }
        }
    }
}
