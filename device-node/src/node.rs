use anyhow::{Error, Result, anyhow};
use shared::{ConnectionStatus, Device, DeviceStatus};
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use zmq::Context;

use networking::Sender;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceAction {
    Start,
    Stop,
    Restart,
}

#[derive(Debug)]
pub enum ControlError {
    InvalidTransition {
        action: DeviceAction,
        status: DeviceStatus,
    },
    Failed(Error),
}

/// Publishes frames captured from any `Device` (camera, microphone, ...) to a hub via
/// a ZMQ `Sender`. Generic over `T: Device` so any producer host can run either device
/// type through the same wiring.
pub struct DeviceNode<T: Device> {
    device: T,
    sender: Arc<Mutex<Sender>>,
    config: Option<T::Config>,
    capture: Option<JoinHandle<Result<(), Error>>>,
}

impl<T: Device> DeviceNode<T> {
    pub fn new(device: T, ip: Ipv4Addr, port: u16, queue_size: u32, data_chunk_size: u32) -> Self {
        DeviceNode {
            device: device,
            config: None,
            capture: None,
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
            config: None,
            capture: None,
            sender: Arc::new(Mutex::new(Sender::new_endpoint(
                endpoint,
                queue_size,
                data_chunk_size,
            ))),
        }
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

impl<T: Device> DeviceNode<T>
where
    T::Config: Clone,
{
    pub fn start(&mut self, device_config: T::Config, context: &Context) -> Result<(), Error> {
        self.sender
            .lock()
            .map_err(|_| anyhow!("sender lock poisoned"))?
            .start(context)?;
        self.config = Some(device_config);
        self.start_capture()
    }

    pub fn apply(&mut self, action: DeviceAction) -> Result<(), ControlError> {
        let status = self.device.device_status().clone();
        let allowed = match action {
            DeviceAction::Start => status == DeviceStatus::Stopped,
            DeviceAction::Stop | DeviceAction::Restart => status == DeviceStatus::Running,
        };
        if !allowed {
            return Err(ControlError::InvalidTransition { action, status });
        }

        let result = match action {
            DeviceAction::Start => self.start_capture(),
            DeviceAction::Stop => self.stop_capture(),
            DeviceAction::Restart => self.restart_capture(),
        };
        result.map_err(ControlError::Failed)
    }

    fn start_capture(&mut self) -> Result<(), Error> {
        let config = self
            .config
            .clone()
            .ok_or_else(|| anyhow!("device has no configuration; start the node first"))?;
        let sender = self.sender.clone();
        let handle = self.device.start(
            config,
            Box::new(move |fp| -> Result<(), Error> {
                let mut locked_sender = sender
                    .lock()
                    .map_err(|_| anyhow!("sender lock poisoned"))?;
                locked_sender.send(fp, zmq::DONTWAIT)?;
                Ok(())
            }),
        )?;
        self.capture = Some(handle);
        Ok(())
    }

    fn stop_capture(&mut self) -> Result<(), Error> {
        let stop_result = self.device.stop();
        if let Some(handle) = self.capture.take() {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::warn!("Capture thread ended with an error: {}", e),
                Err(_) => tracing::warn!("Capture thread panicked"),
            }
        }
        stop_result
    }

    fn restart_capture(&mut self) -> Result<(), Error> {
        self.stop_capture()?;
        self.start_capture()
    }
}
