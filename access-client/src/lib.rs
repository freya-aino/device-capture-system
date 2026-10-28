use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::{Error, Result};
use networking::Receiver;
use shared::{AlignedGroup, Aligner, FramePacket, Sample};

/// Consumer-side client: connects a `Receiver` to a `control-hub`'s XPUB endpoint and
/// exposes the aggregated frame stream through a simple blocking API. This is the
/// foundation future bindings (e.g. a Python client, per README/SPECS §5) sit on top of.
pub struct AccessClient {
    receiver: Receiver,
    context: zmq::Context,
}

impl AccessClient {
    pub fn new(hub_ip: Ipv4Addr, hub_port: u16, queue_size: u32, data_chunk_size: u32) -> Self {
        AccessClient {
            receiver: Receiver::new(hub_ip, hub_port, queue_size, data_chunk_size),
            context: zmq::Context::new(),
        }
    }

    pub fn new_endpoint(
        endpoint: impl Into<String>,
        queue_size: u32,
        data_chunk_size: u32,
    ) -> Self {
        AccessClient {
            receiver: Receiver::new_endpoint(endpoint, queue_size, data_chunk_size),
            context: zmq::Context::new(),
        }
    }

    pub fn connect(&mut self) -> Result<(), Error> {
        self.receiver.start(&self.context)
    }

    pub fn disconnect(&mut self) -> Result<(), Error> {
        self.receiver.stop()
    }

    /// Blocks (polling) until a frame is available or `timeout` elapses.
    pub fn next_frame(&mut self, timeout: Duration) -> Result<Option<FramePacket>, Error> {
        let start = std::time::Instant::now();
        loop {
            if let Some(packet) = self.receiver.receive(zmq::DONTWAIT)? {
                return Ok(Some(packet));
            }
            if start.elapsed() >= timeout {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Calls `callback` for every received frame until it returns `false`.
    pub fn run(&mut self, mut callback: impl FnMut(FramePacket) -> bool) -> Result<(), Error> {
        loop {
            match self.receiver.receive(zmq::DONTWAIT)? {
                Some(packet) => {
                    if !callback(packet) {
                        return Ok(());
                    }
                }
                None => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    }
}

/// Wraps an `AccessClient` and groups frames from several producers into aligned sets by capture
/// time. Each group carries the original `FramePacket`s, so no payload is dropped.
pub struct AlignedClient {
    client: AccessClient,
    aligner: Aligner<FramePacket>,
}

impl AlignedClient {
    pub fn new(client: AccessClient, devices: Vec<String>, tolerance: Duration) -> Self {
        AlignedClient {
            client,
            aligner: Aligner::new(devices, tolerance),
        }
    }

    pub fn connect(&mut self) -> Result<(), Error> {
        self.client.connect()
    }

    pub fn disconnect(&mut self) -> Result<(), Error> {
        self.client.disconnect()
    }

    pub fn dropped(&self) -> u64 {
        self.aligner.dropped()
    }

    /// Blocks until an aligned group is complete or `timeout` elapses.
    pub fn next_group(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<AlignedGroup<FramePacket>>, Error> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() >= timeout {
                return Ok(None);
            }
            let remaining = timeout.saturating_sub(start.elapsed());
            let Some(packet) = self.client.next_frame(remaining)? else {
                return Ok(None);
            };
            let Some(time) = packet.frame_info.tx_timestamp else {
                continue;
            };
            let sample = Sample {
                device_id: packet.frame_info.device_info.id.clone(),
                time,
                payload: packet,
            };
            if let Some(group) = self.aligner.push(sample) {
                return Ok(Some(group));
            }
        }
    }
}
