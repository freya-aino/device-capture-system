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

                let mut frame_information = FramePacketInformation::deserialize(&parts[0]).unwrap();
                assert!(
                    frame_information.tx_timestamp.is_some(),
                    "tx_timestamp is not set"
                );
                frame_information.rx_timestamp = Some(SystemTime::now());
                let data = parts[1..].concat().into_boxed_slice();

                self.0.status = ConnectionStatus::Active;
                self.0.update_stats(data.len() as u64);

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
