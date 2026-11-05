use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, spawn};

use anyhow::{Error, Result, anyhow};
use zmq::Error::EAGAIN;
use zmq::{Context, POLLIN, PUB, Socket};

use shared::{FramePacket, FramePacketInformation};

/// Default output queue of a `FrameRelay`, in frames.
pub const DEFAULT_RELAY_QUEUE: u32 = 4;

/// Input queue of a `FrameRelay`, in frames. Generous on purpose: ZMQ stops reading from the network
/// when this queue is full, and then the newest frames would wait at the producer. The relay drains
/// the queue before every transform, so it holds about one transform's worth of frames.
const INPUT_QUEUE: i32 = 256;

/// What a `FrameRelay` does with each frame: return the frame to publish, `None` to skip it, or an
/// error, which is logged and drops that frame only.
pub type FrameTransform = Box<dyn FnMut(FramePacket) -> Result<Option<FramePacket>> + Send>;

/// Hub-side per-frame processing. Subscribes to the producer endpoints, passes frames through a
/// transform, and publishes the results on its own PUB endpoint in the normal frame format, so a
/// plain `Receiver` reads them. The frame's sequence part is kept, so gap counting still works. It
/// runs beside the forwarding proxy and leaves the raw stream untouched.
///
/// It is a live relay, not a queue. Before each transform it drains every frame waiting at its input
/// and keeps only the newest per device, so a transform slower than the producers lowers the output
/// rate instead of adding latency. Its output queue is short, so a slow consumer loses frames rather
/// than making the hub buffer them. Consumers see both kinds of drop as sequence gaps.
pub struct FrameRelay {
    producers: Vec<String>,
    output_endpoint: String,
    queue: i32,
    transform: Option<FrameTransform>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<Result<(), Error>>>,
}

impl FrameRelay {
    pub fn new(
        producers: Vec<String>,
        output_endpoint: impl Into<String>,
        transform: FrameTransform,
    ) -> Self {
        FrameRelay {
            producers,
            output_endpoint: output_endpoint.into(),
            queue: DEFAULT_RELAY_QUEUE as i32,
            transform: Some(transform),
            stop: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    /// How many processed frames may wait for a slow consumer before new ones are dropped.
    pub fn with_queue(mut self, frames: u32) -> Self {
        self.queue = frames.max(1) as i32;
        self
    }

    pub fn start(&mut self, context: &Context) -> Result<(), Error> {
        let transform = self
            .transform
            .take()
            .ok_or_else(|| anyhow!("frame relay was already started"))?;

        let input = context.socket(zmq::SUB)?;
        input.set_subscribe(&[])?;
        input.set_linger(0)?;
        input.set_rcvhwm(INPUT_QUEUE)?;
        for producer in &self.producers {
            input.connect(producer)?;
        }

        let output = context.socket(PUB)?;
        output.set_linger(0)?;
        output.set_sndhwm(self.queue)?;
        output.bind(&self.output_endpoint)?;

        let stop = self.stop.clone();
        self.handle = Some(spawn(move || run_relay(input, output, transform, stop)));
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow!("frame relay thread panicked"))??;
        }
        Ok(())
    }
}

fn run_relay(
    input: Socket,
    output: Socket,
    mut transform: FrameTransform,
    stop: Arc<AtomicBool>,
) -> Result<(), Error> {
    while !stop.load(Ordering::SeqCst) {
        if input.poll(POLLIN, 100)? == 0 {
            continue;
        }
        for (packet, sequence) in drain_newest(&input)? {
            let device = packet.frame_info.device_info.id.clone();
            let out = match transform(packet) {
                Ok(Some(out)) => out,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!("Dropping frame from {:?} in frame relay: {:#}", device, e);
                    continue;
                }
            };
            // Chunk boundaries carry no meaning (`shared/README.md`), and one part per payload
            // avoids thousands of small parts for a processed frame of several megabytes.
            let header = out.frame_info.serialize()?;
            let message: [&[u8]; 3] = [&header, &sequence, &out.data];
            output.send_multipart(message, 0)?;
        }
    }
    Ok(())
}

/// Reads every frame waiting at the input and returns the newest one per device, in the order the
/// devices were first seen.
fn drain_newest(input: &Socket) -> Result<Vec<(FramePacket, Vec<u8>)>, Error> {
    let mut newest: Vec<(FramePacket, Vec<u8>)> = Vec::new();
    loop {
        let parts = match input.recv_multipart(zmq::DONTWAIT) {
            Ok(parts) => parts,
            Err(EAGAIN) => return Ok(newest),
            Err(e) => return Err(anyhow!("frame relay receive failed: {}", e)),
        };
        let Some(frame) = frame_of(parts) else {
            continue;
        };
        let id = &frame.0.frame_info.device_info.id;
        match newest
            .iter_mut()
            .find(|(p, _)| &p.frame_info.device_info.id == id)
        {
            Some(slot) => *slot = frame,
            None => newest.push(frame),
        }
    }
}

/// Decodes a raw frame message into a packet and its sequence part.
fn frame_of(mut parts: Vec<Vec<u8>>) -> Option<(FramePacket, Vec<u8>)> {
    if parts.len() < 3 {
        tracing::warn!("Dropping frame with {} parts in frame relay.", parts.len());
        return None;
    }
    let info = match FramePacketInformation::deserialize(&parts[0]) {
        Ok(info) => info,
        Err(e) => {
            tracing::warn!(
                "Dropping frame with undecodable header in frame relay: {}",
                e
            );
            return None;
        }
    };
    let sequence = std::mem::take(&mut parts[1]);
    let data = parts[2..].concat().into_boxed_slice();
    Some((FramePacket::new(info, data), sequence))
}
