//! The hub input: single frames from the raw or processed port, or complete groups from the sync
//! port.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use networking::GroupReceiver;
use shared::FramePacket;

use access_client::AccessClient;

use crate::{RecorderConfig, Source};

/// A frame with the wire sequence number it arrived with.
pub struct Received {
    pub packet: FramePacket,
    pub sequence: Option<u64>,
}

pub enum Input {
    Frames(AccessClient),
    Groups {
        receiver: GroupReceiver,
        _context: zmq::Context,
        devices: HashSet<String>,
        incomplete: u64,
    },
}

/// Receive queue and part size for the hub connection.
const QUEUE_SIZE: u32 = 256;
const CHUNK_SIZE: u32 = 1024;

impl Input {
    pub fn connect(config: &RecorderConfig) -> Result<Self> {
        let endpoint = config.hub_endpoint();
        match config.source {
            Source::Raw | Source::Processed => {
                let mut client = AccessClient::new_endpoint(endpoint, QUEUE_SIZE, CHUNK_SIZE);
                client.connect()?;
                Ok(Input::Frames(client))
            }
            Source::Aligned => {
                let context = zmq::Context::new();
                let mut receiver = GroupReceiver::new_endpoint(endpoint, QUEUE_SIZE, CHUNK_SIZE);
                receiver.start(&context)?;
                Ok(Input::Groups {
                    receiver,
                    _context: context,
                    devices: config.devices.iter().cloned().collect(),
                    incomplete: 0,
                })
            }
        }
    }

    /// Groups seen on the sync port that lacked a recorded device and were therefore skipped.
    pub fn incomplete_groups(&self) -> u64 {
        match self {
            Input::Frames(_) => 0,
            Input::Groups { incomplete, .. } => *incomplete,
        }
    }

    /// The next frame(s) to record, or an empty list when `timeout` elapsed first. Frames from
    /// devices that are not recorded are still returned here; the recorder ignores them. From the
    /// sync port, only groups containing every recorded device are returned, trimmed to those
    /// devices.
    pub fn next(&mut self, timeout: Duration) -> Result<Vec<Received>> {
        match self {
            Input::Frames(client) => Ok(client
                .next_frame(timeout)?
                .map(|packet| {
                    let sequence = client.last_sequence(&packet.frame_info.device_info.id);
                    vec![Received { packet, sequence }]
                })
                .unwrap_or_default()),
            Input::Groups {
                receiver,
                devices,
                incomplete,
                ..
            } => {
                let start = Instant::now();
                loop {
                    if let Some(group) = receiver.receive(zmq::DONTWAIT)? {
                        let present: HashSet<&str> = group
                            .frames
                            .iter()
                            .map(|f| f.frame_info.device_info.id.as_str())
                            .collect();
                        if devices.iter().all(|d| present.contains(d.as_str())) {
                            return Ok(group
                                .frames
                                .into_iter()
                                .zip(group.sequences)
                                .filter(|(f, _)| devices.contains(&f.frame_info.device_info.id))
                                .map(|(packet, sequence)| Received {
                                    packet,
                                    sequence: Some(sequence),
                                })
                                .collect());
                        }
                        *incomplete += 1;
                    }
                    if start.elapsed() >= timeout {
                        return Ok(Vec::new());
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    pub fn disconnect(&mut self) -> Result<()> {
        match self {
            Input::Frames(client) => client.disconnect(),
            Input::Groups { receiver, .. } => receiver
                .stop()
                .map_err(|e| anyhow!("closing the sync-port receiver: {e}")),
        }
    }
}
