use anyhow::{Error, Result};
use shared::ConnectionStatus;
use std::net::Ipv4Addr;
use zmq::{Context, XPUB, XSUB};

use super::Connection;

pub struct Proxy {
    from: Connection,
    to: Connection,
}

impl Proxy {
    pub fn new(
        from_ip: Ipv4Addr,
        from_port: u16,
        to_ip: Ipv4Addr,
        to_port: u16,
        queue_size: u32,
        data_chunk_size: u32,
    ) -> Self {
        Proxy {
            from: Connection::new(from_ip, from_port, queue_size, data_chunk_size),
            to: Connection::new(to_ip, to_port, queue_size, data_chunk_size),
        }
    }

    pub fn connection_status(&self) -> (&ConnectionStatus, &ConnectionStatus) {
        (&self.from.status, &self.to.status)
    }

    pub fn data_chunk_size(&self) -> u32 {
        self.from.data_chunk_size
    }

    pub fn start(&mut self, context: &Context) -> Result<(), Error> {
        self.from.open(context, XSUB)?;
        self.to.open(context, XPUB)?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.from.close()?;
        self.to.close()?;
        Ok(())
    }
}
