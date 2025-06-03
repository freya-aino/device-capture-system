use anyhow::{Error, Result};
use devices::AlsaMicrophoneDevice;
use shared::{ConnectionStatus, Device, DeviceStatus, MicrophoneConfig};
use std::sync::{Arc, Mutex};
use zmq::Context;

use networking::Sender;

pub struct DeviceNode<T: Device> {
    device: T,
    sender: Arc<Mutex<Sender>>,
}

impl DeviceNode<AlsaMicrophoneDevice> {
    pub fn new(device: AlsaMicrophoneDevice, sender: Sender) -> Self {
        DeviceNode {
            device,
            sender: Arc::new(Mutex::new(sender)),
        }
    }

    pub fn start(
        &mut self,
        device_config: MicrophoneConfig,
        context: &Context,
    ) -> Result<(), Error> {
        let sender = self.sender.clone();

        sender.lock().unwrap().start(context).unwrap();
        let data_chunk_size = sender.lock().unwrap().data_chunk_size();

        self.device.start(
            device_config,
            Box::new(move |fp| -> Result<(), Error> {
                let mut locked_sender = sender.lock().unwrap();
                locked_sender.send(fp, zmq::DONTWAIT, data_chunk_size)?;
                Ok(())
            }),
        )?;

        Ok(())
    }

    pub fn status(&self) -> Result<(DeviceStatus, ConnectionStatus), Error> {
        let device_status = self.device.device_status().clone();
        let connection_status = self.sender.lock().unwrap().connection_status().clone();

        Ok((device_status, connection_status))
    }
}
