use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, spawn};

use anyhow::{Error, Result, anyhow};
use zmq::Error::EAGAIN;
use zmq::{Context, POLLIN, PUB, Socket};

use shared::{FramePacket, FramePacketInformation};

/// What a `FrameRelay` does with each frame: return the frame to publish, `None` to skip it, or an
/// error, which is logged and drops that frame only.
pub type FrameTransform = Box<dyn FnMut(FramePacket) -> Result<Option<FramePacket>> + Send>;

/// Hub-side per-frame processing. Subscribes to the producer endpoints, passes every frame through
/// a transform, and publishes the results on its own PUB endpoint in the normal frame format, so a
/// plain `Receiver` reads them. The frame's sequence part is kept, so gap counting still works. It
/// runs beside the forwarding proxy and leaves the raw stream untouched.
pub struct FrameRelay {
    producers: Vec<String>,
    output_endpoint: String,
    data_chunk_size: usize,
    transform: Option<FrameTransform>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<Result<(), Error>>>,
}

impl FrameRelay {
    pub fn new(
        producers: Vec<String>,
        output_endpoint: impl Into<String>,
        data_chunk_size: u32,
        transform: FrameTransform,
    ) -> Self {
        assert!(
            data_chunk_size > 0,
            "Data chunk size must be greater than 0."
        );
        FrameRelay {
            producers,
            output_endpoint: output_endpoint.into(),
            data_chunk_size: data_chunk_size as usize,
            transform: Some(transform),
            stop: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    pub fn start(&mut self, context: &Context) -> Result<(), Error> {
        let transform = self
            .transform
            .take()
            .ok_or_else(|| anyhow!("frame relay was already started"))?;

        let input = context.socket(zmq::SUB)?;
        input.set_subscribe(&[])?;
        input.set_linger(0)?;
        for producer in &self.producers {
            input.connect(producer)?;
        }

        let output = context.socket(PUB)?;
        output.set_linger(0)?;
        output.bind(&self.output_endpoint)?;

        let stop = self.stop.clone();
        let chunk = self.data_chunk_size;
        self.handle = Some(spawn(move || {
            run_relay(input, output, transform, chunk, stop)
        }));
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
    data_chunk_size: usize,
    stop: Arc<AtomicBool>,
) -> Result<(), Error> {
    while !stop.load(Ordering::SeqCst) {
        if input.poll(POLLIN, 100)? == 0 {
            continue;
        }
        let parts = match input.recv_multipart(zmq::DONTWAIT) {
            Ok(parts) => parts,
            Err(EAGAIN) => continue,
            Err(e) => return Err(anyhow!("frame relay receive failed: {}", e)),
        };
        let Some((packet, sequence)) = frame_of(parts) else {
            continue;
        };
        let device = packet.frame_info.device_info.id.clone();
        let out = match transform(packet) {
            Ok(Some(out)) => out,
            Ok(None) => continue,
            Err(e) => {
                tracing::warn!("Dropping frame from {:?} in frame relay: {:#}", device, e);
                continue;
            }
        };

        let header = out.frame_info.serialize()?;
        let mut message: Vec<&[u8]> = vec![&header, &sequence];
        message.extend(out.data.chunks(data_chunk_size));
        output.send_multipart(&message, 0)?;
    }
    Ok(())
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
