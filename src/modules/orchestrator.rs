use std::net::Ipv4Addr;

use anyhow::{Error, Result};


use crate::DeviceManager;
use crate::Receiver;
use crate::Sender;
use crate::IOManager;



pub struct DeviceSender {
    device_manager: DeviceManager,
    sender: Sender,
}

impl DeviceSender {
    pub fn new(device_manager: DeviceManager, sender: Sender) -> Self {
        DeviceSender {
            device_manager: device_manager,
            sender: sender
        }
    }
}


pub struct DeviceReceiver {
    io_manager: IOManager,
    receiver: Receiver,
}

impl DeviceReceiver {
    pub fn new(io_manager: IOManager, receiver: Receiver) -> Self {
        DeviceReceiver {
            io_manager: io_manager,
            receiver: receiver
        }
    }
}

