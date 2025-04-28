use std::net::Ipv4Addr;
use std::sync::mpsc;
use std::thread;

use anyhow::{Error, Result};


use crate::DeviceManager;
use crate::Receiver;
use crate::Sender;
use crate::FramePacket;


pub enum ThreadControlMessage {
    Start,
    Stop,
    Pause,
    Resume,
    Terminate,
}

pub struct ThreadHandle {
    thread: std::thread::JoinHandle<()>,
    control_tx: mpsc::Sender<ThreadControlMessage>,
}

pub struct DeviceSender {
    device_manager: DeviceManager,
    sender: Sender,
    thread_handle: Option<ThreadHandle>,
}

impl DeviceSender {
    pub fn new(device_manager: DeviceManager, sender: Sender) -> Self {

        DeviceSender {
            device_manager: device_manager,
            sender: sender,
            thread_handle: None,
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        
        let (data_tx, data_rx) = flume::bounded::<FramePacket>(32);
        let (ctrl_tx, ctrl_rx) = flume::unbounded::<ThreadControlMessage>();

        
        
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

