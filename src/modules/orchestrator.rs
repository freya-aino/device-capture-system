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
    pub fn new(device_manager: DeviceManager, ip: Ipv4Addr, port: u16, queue_size: u32) -> Self {
        DeviceSender {
            device_manager: device_manager,
            sender: Sender::new(ip, port, queue_size),
        }
    }
}


pub struct DeviceReceiver {
    io_manager: IOManager,
    receiver: Receiver,
}

impl DeviceReceiver {
    pub fn new(io_manager: IOManager, ip: Ipv4Addr, port: u16, queue_size: u32) -> Self {
        DeviceReceiver {
            io_manager: io_manager,
            receiver: Receiver::new(ip, port, queue_size),
        }
    }
}