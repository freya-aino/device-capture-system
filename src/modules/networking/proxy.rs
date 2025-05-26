
use zmq::{XPUB, XSUB};
use std::net::Ipv4Addr;

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
    ) -> Self {
        Proxy {
            from: Connection::new(from_ip, from_port, queue_size),
            to: Connection::new(to_ip, to_port, queue_size),
        }
    }

    pub fn initialize(&mut self, context: &Context) -> Result<(), Error> {
        self.from.initialize(context, XSUB)?;
        self.to.initialize(context, XPUB)?;
        Ok(())
    }
}