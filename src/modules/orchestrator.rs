use std::net::Ipv4Addr;

use anyhow::{Error, Result};

use crate::Device;
use crate::FramePacket;
use crate::Receiver;
use crate::Sender;

pub enum ThreadControlMessage {
    Start,
    Stop,
    // Pause,
    // Resume,
    // Terminate,
}

// pub struct ThreadHandle {
//     thread: std::thread::JoinHandle<()>,
//     control_tx: flume::Sender<ThreadControlMessage>,
//     // data_rx: flume::Receiver<FramePacket>,
// }

pub struct DeviceSender {
    // device_manager: DeviceManager,
    device: Device,
    sender: Sender,
}

impl DeviceSender {
    pub fn new(device: Device, ip: Ipv4Addr, port: u16, queue_size: u32) -> Self {
        DeviceSender {
            device: device,
            sender: Sender::new(ip, port, queue_size),
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

// pub struct DeviceReceiver {
//     io_manager: IOManager,
//     receiver: Receiver,
// }

// impl DeviceReceiver {
//     pub fn new(io_manager: IOManager, receiver: Receiver) -> Self {
//         DeviceReceiver {
//             io_manager: io_manager,
//             receiver: receiver
//         }
//     }
// }
