use anyhow::{Error, Result};
use shared::ConnectionStatus;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;
use zmq::{Context, PAIR, Socket, XPUB, XSUB};

use super::Connection;

static CONTROL_ENDPOINT_ID: AtomicUsize = AtomicUsize::new(0);

/// `Proxy` runs a bidirectional ZMQ forwarding device, fanning in frames published by
/// producers connected to its `from` (XSUB) endpoint and fanning them out to consumers
/// connected to its `to` (XPUB) endpoint. The actual forwarding happens on a dedicated
/// background thread via `zmq::proxy_steerable`, which also lets `stop()` ask the proxy
/// to terminate cleanly instead of leaking the thread.
pub struct Proxy {
    from: Connection,
    to: Connection,
    control_endpoint: String,
    control: Option<Socket>,
    handle: Option<JoinHandle<Result<(), Error>>>,
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
            control_endpoint: format!("inproc://proxy-control-{}-{}", from_port, to_port),
            control: None,
            handle: None,
        }
    }

    pub fn new_endpoints(
        from_endpoint: impl Into<String>,
        to_endpoint: impl Into<String>,
        queue_size: u32,
        data_chunk_size: u32,
    ) -> Self {
        let id = CONTROL_ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
        Proxy {
            from: Connection::new_endpoint(from_endpoint, queue_size, data_chunk_size),
            to: Connection::new_endpoint(to_endpoint, queue_size, data_chunk_size),
            control_endpoint: format!("inproc://proxy-control-{}", id),
            control: None,
            handle: None,
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

        // The control socket pair lets `stop()` signal the forwarding thread to
        // terminate. It must be bound before the connecting side is created.
        let control_listener = context.socket(PAIR)?;
        control_listener.bind(&self.control_endpoint)?;
        let control_worker = context.socket(PAIR)?;
        control_worker.connect(&self.control_endpoint)?;

        let mut frontend = self
            .from
            .socket
            .take()
            .ok_or_else(|| Error::msg("'from' socket missing after open()"))?;
        let mut backend = self
            .to
            .socket
            .take()
            .ok_or_else(|| Error::msg("'to' socket missing after open()"))?;

        let handle = std::thread::spawn(move || -> Result<(), Error> {
            let mut control = control_worker;
            zmq::proxy_steerable(&mut frontend, &mut backend, &mut control)
                .map_err(|e| Error::msg(format!("Proxy terminated with error: {}", e)))
        });

        self.control = Some(control_listener);
        self.handle = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        if let Some(control) = self.control.take() {
            control.send("TERMINATE", 0)?;
        }
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| Error::msg("Proxy thread panicked"))??;
        }
        self.from.status = ConnectionStatus::Closed;
        self.to.status = ConnectionStatus::Closed;
        Ok(())
    }
}
