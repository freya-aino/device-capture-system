use anyhow::{Error, Result, anyhow};
use shared::{ConnectionStatus, Device, DeviceStatus};
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};
use zmq::Context;

use networking::Sender;

/// Publishes frames captured from any `Device` (camera, microphone, ...) to a hub via
/// a ZMQ `Sender`. Generic over `T: Device` so any producer host can run either device
/// type through the same wiring.
pub struct DeviceNode<T: Device> {
    device: T,
    sender: Arc<Mutex<Sender>>,
}

impl<T: Device> DeviceNode<T> {
    pub fn new(device: T, ip: Ipv4Addr, port: u16, queue_size: u32, data_chunk_size: u32) -> Self {
        DeviceNode {
            device: device,
            sender: Arc::new(Mutex::new(Sender::new(
                ip,
                port,
                queue_size,
                data_chunk_size,
            ))),
        }
    }

    pub fn new_endpoint(
        device: T,
        endpoint: impl Into<String>,
        queue_size: u32,
        data_chunk_size: u32,
    ) -> Self {
        DeviceNode {
            device,
            sender: Arc::new(Mutex::new(Sender::new_endpoint(
                endpoint,
                queue_size,
                data_chunk_size,
            ))),
        }
    }

    pub fn start(&mut self, device_config: T::Config, context: &Context) -> Result<(), Error> {
        let sender = self.sender.clone();

        sender
            .lock()
            .map_err(|_| anyhow!("sender lock poisoned"))?
            .start(context)?;
        self.device.start(
            device_config,
            Box::new(move |fp| -> Result<(), Error> {
                let mut locked_sender = sender
                    .lock()
                    .map_err(|_| anyhow!("sender lock poisoned"))?;
                locked_sender.send(fp, zmq::DONTWAIT)?;
                Ok(())
            }),
        )?;

        Ok(())
    }

    pub fn status(&self) -> Result<(DeviceStatus, ConnectionStatus), Error> {
        let device_status = self.device.device_status().clone();
        let connection_status = self
            .sender
            .lock()
            .map_err(|_| anyhow!("sender lock poisoned"))?
            .connection_status()
            .clone();

        Ok((device_status, connection_status))
    }
}
