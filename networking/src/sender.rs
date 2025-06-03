use anyhow::{Error, Result};
use std::{net::Ipv4Addr, time::SystemTime};
use zmq::{Context, PUB};

use super::Connection;
use shared::{ConnectionStatus, FramePacket};

// #[derive(Clone)]
pub struct Sender(Connection);

impl Sender {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32, data_chunk_size: u32) -> Self {
        Sender(Connection::new(
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
        self.0.open(context, PUB)
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.0.close()
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

        let mut frame_info = frame_packet.frame_info.clone();
        frame_info.tx_timestamp = Some(SystemTime::now());

        let serialized_info = frame_info.serialize()?;
        let data = frame_packet.data.as_ref();

        let mut chunked_message: Vec<&[u8]> = Vec::new();

        // append data to chunked message
        chunked_message.push(&serialized_info);
        for chunk in data.chunks(data_chunk_size as usize) {
            chunked_message.push(chunk);
        }

        self.0
            .get_socket()?
            .send_multipart(&chunked_message, zmq_flags)?;

        self.0.status = ConnectionStatus::Active;
        self.0
            .update_stats((serialized_info.len() + frame_packet.data.len()) as u64);
        return Ok(());
    }
}
