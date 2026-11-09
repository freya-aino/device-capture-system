use anyhow::{Error, Result};
use std::{net::Ipv4Addr, time::SystemTime};
use zmq::{Context, PUB};

use super::Connection;
use shared::{ConnectionStatus, FramePacket};

// #[derive(Clone)]
pub struct Sender {
    connection: Connection,
    next_sequence: u64,
}

impl Sender {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32, data_chunk_size: u32) -> Self {
        Sender {
            connection: Connection::new(host_address, port, queue_size, data_chunk_size),
            next_sequence: 0,
        }
    }

    pub fn new_endpoint(
        endpoint: impl Into<String>,
        queue_size: u32,
        data_chunk_size: u32,
    ) -> Self {
        Sender {
            connection: Connection::new_endpoint(endpoint, queue_size, data_chunk_size),
            next_sequence: 0,
        }
    }

    pub fn connection_status(&self) -> &ConnectionStatus {
        &self.connection.status
    }

    pub fn data_chunk_size(&self) -> u32 {
        self.connection.data_chunk_size
    }

    pub fn start(&mut self, context: &Context) -> Result<(), Error> {
        self.connection.open(context, PUB)
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.connection.close()
    }

    pub fn send(&mut self, frame_packet: FramePacket, zmq_flags: i32) -> Result<(), Error> {
        let data_chunk_size = self.connection.data_chunk_size;
        assert!(
            data_chunk_size > 0,
            "Data chunk size must be greater than 0."
        );

        let mut frame_info = frame_packet.frame_info.clone();
        frame_info.tx_timestamp = Some(SystemTime::now());

        let serialized_info = frame_info.serialize()?;
        let serialized_sequence = self.next_sequence.to_le_bytes();
        let data = frame_packet.data.as_ref();

        let mut chunked_message: Vec<&[u8]> = Vec::new();

        // append sequence number and data to chunked message
        chunked_message.push(&serialized_info);
        chunked_message.push(&serialized_sequence);
        for chunk in data.chunks(data_chunk_size as usize) {
            chunked_message.push(chunk);
        }

        self.connection
            .get_socket()?
            .send_multipart(&chunked_message, zmq_flags)?;

        self.next_sequence += 1;
        self.connection.status = ConnectionStatus::Active;
        self.connection
            .update_stats((serialized_info.len() + frame_packet.data.len()) as u64);
        return Ok(());
    }
}
