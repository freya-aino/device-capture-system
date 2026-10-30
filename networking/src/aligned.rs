use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, spawn};
use std::time::{Duration, SystemTime};

use anyhow::{Error, Result, anyhow};
use zmq::Error::EAGAIN;
use zmq::{Context, POLLIN, PUB, SUB, Socket};

use super::Connection;
use shared::{AlignedGroup, Aligner, FramePacket, FramePacketInformation, Sample};

/// One aligned set of frames received from an `AlignedRelay`.
#[derive(Debug, PartialEq, Clone)]
pub struct FrameGroup {
    pub frames: Vec<FramePacket>,
    pub skew: Duration,
}

/// Encodes an aligned group as one multipart message. The first part is a header of little-endian
/// integers: the skew in nanoseconds (u64), the sample count (u32) and the number of parts of each
/// sample (u32 each). The original multipart parts of every sample follow unchanged.
fn encode_group(group: AlignedGroup<Vec<Vec<u8>>>) -> Vec<Vec<u8>> {
    let mut header = Vec::with_capacity(12 + 4 * group.samples.len());
    header.extend_from_slice(&(group.skew.as_nanos() as u64).to_le_bytes());
    header.extend_from_slice(&(group.samples.len() as u32).to_le_bytes());
    for sample in &group.samples {
        header.extend_from_slice(&(sample.payload.len() as u32).to_le_bytes());
    }

    let mut parts = vec![header];
    for sample in group.samples {
        parts.extend(sample.payload);
    }
    parts
}

/// Decodes a message produced by `encode_group`, rejecting it if the header and body disagree.
pub fn decode_group(parts: &[Vec<u8>]) -> Result<FrameGroup, Error> {
    let header = parts
        .first()
        .ok_or_else(|| anyhow!("aligned group message has no header"))?;
    if header.len() < 12 {
        return Err(anyhow!("aligned group header is too short"));
    }

    let skew_bytes: [u8; 8] = header[0..8].try_into()?;
    let count_bytes: [u8; 4] = header[8..12].try_into()?;
    let skew = Duration::from_nanos(u64::from_le_bytes(skew_bytes));
    let count = u32::from_le_bytes(count_bytes) as usize;
    if header.len() != 12 + 4 * count {
        return Err(anyhow!(
            "aligned group header does not match its sample count"
        ));
    }

    let mut part_counts = Vec::with_capacity(count);
    for chunk in header[12..].chunks_exact(4) {
        let bytes: [u8; 4] = chunk.try_into()?;
        part_counts.push(u32::from_le_bytes(bytes) as usize);
    }

    let body = &parts[1..];
    if part_counts.iter().sum::<usize>() != body.len() {
        return Err(anyhow!("aligned group body does not match its header"));
    }

    let mut frames = Vec::with_capacity(count);
    let mut offset = 0;
    for len in part_counts {
        frames.push(decode_frame(&body[offset..offset + len])?);
        offset += len;
    }
    Ok(FrameGroup { frames, skew })
}

fn decode_frame(parts: &[Vec<u8>]) -> Result<FramePacket, Error> {
    if parts.len() < 3 {
        return Err(anyhow!("frame needs at least 3 parts, got {}", parts.len()));
    }
    let mut frame_info = FramePacketInformation::deserialize(&parts[0])?;
    frame_info.rx_timestamp = Some(SystemTime::now());
    let data = parts[2..].concat().into_boxed_slice();
    Ok(FramePacket::new(frame_info, data))
}

/// Consumer side of an `AlignedRelay`: receives aligned groups from the relay's PUB endpoint.
pub struct GroupReceiver {
    connection: Connection,
}

impl GroupReceiver {
    pub fn new_endpoint(
        endpoint: impl Into<String>,
        queue_size: u32,
        data_chunk_size: u32,
    ) -> Self {
        GroupReceiver {
            connection: Connection::new_endpoint(endpoint, queue_size, data_chunk_size),
        }
    }

    pub fn start(&mut self, context: &Context) -> Result<(), Error> {
        self.connection.open(context, SUB)
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.connection.close()
    }

    /// Returns the next aligned group, or `None` when nothing is pending or the message is malformed.
    pub fn receive(&mut self, zmq_flags: i32) -> Result<Option<FrameGroup>, Error> {
        let socket = self.connection.get_socket()?;
        match socket.recv_multipart(zmq_flags) {
            Ok(parts) => match decode_group(&parts) {
                Ok(group) => Ok(Some(group)),
                Err(e) => {
                    tracing::warn!("Dropping malformed aligned group: {}", e);
                    Ok(None)
                }
            },
            Err(EAGAIN) => Ok(None),
            Err(e) => Err(Error::msg(format!("Error receiving aligned group: {}", e))),
        }
    }
}

/// Hub-side sync. Subscribes to the producer endpoints, groups their frames by capture time with
/// an `Aligner`, and publishes every aligned group on its own PUB endpoint. It runs beside the
/// forwarding proxy and leaves the raw stream untouched.
pub struct AlignedRelay {
    producers: Vec<String>,
    output_endpoint: String,
    devices: Vec<String>,
    tolerance: Duration,
    max_wait: Duration,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<Result<(), Error>>>,
}

impl AlignedRelay {
    pub fn new(
        producers: Vec<String>,
        output_endpoint: impl Into<String>,
        devices: Vec<String>,
        tolerance: Duration,
    ) -> Self {
        AlignedRelay {
            producers,
            output_endpoint: output_endpoint.into(),
            devices,
            tolerance,
            max_wait: shared::DEFAULT_MAX_WAIT.max(tolerance),
            stop: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    /// How long the relay holds a frame for partners from the other devices (see
    /// `Aligner::with_max_wait`). Must cover the transport-latency difference between devices.
    pub fn with_max_wait(mut self, max_wait: Duration) -> Self {
        self.max_wait = max_wait.max(self.tolerance);
        self
    }

    pub fn start(&mut self, context: &Context) -> Result<(), Error> {
        if self.handle.is_some() {
            return Err(anyhow!("aligned relay is already running"));
        }

        let input = context.socket(SUB)?;
        input.set_subscribe(&[])?;
        input.set_linger(0)?;
        for producer in &self.producers {
            input.connect(producer)?;
        }

        let output = context.socket(PUB)?;
        output.set_linger(0)?;
        output.bind(&self.output_endpoint)?;

        let aligner =
            Aligner::new(self.devices.clone(), self.tolerance).with_max_wait(self.max_wait);
        self.stop = Arc::new(AtomicBool::new(false));
        let stop = self.stop.clone();
        self.handle = Some(spawn(move || run_relay(input, output, aligner, stop)));
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow!("aligned relay thread panicked"))??;
        }
        Ok(())
    }
}

fn run_relay(
    input: Socket,
    output: Socket,
    mut aligner: Aligner<Vec<Vec<u8>>>,
    stop: Arc<AtomicBool>,
) -> Result<(), Error> {
    while !stop.load(Ordering::SeqCst) {
        if input.poll(POLLIN, 100)? == 0 {
            continue;
        }
        let parts = match input.recv_multipart(zmq::DONTWAIT) {
            Ok(parts) => parts,
            Err(EAGAIN) => continue,
            Err(e) => return Err(anyhow!("aligned relay receive failed: {}", e)),
        };
        let Some(sample) = sample_of(parts) else {
            continue;
        };
        if let Some(group) = aligner.push(sample) {
            let encoded = encode_group(group);
            let refs: Vec<&[u8]> = encoded.iter().map(|part| part.as_slice()).collect();
            output.send_multipart(&refs, 0)?;
        }
    }
    Ok(())
}

/// Reads the device and capture time from a raw frame message without decoding its payload.
fn sample_of(parts: Vec<Vec<u8>>) -> Option<Sample<Vec<Vec<u8>>>> {
    if parts.len() < 3 {
        tracing::warn!(
            "Dropping frame with {} parts in aligned relay.",
            parts.len()
        );
        return None;
    }
    let info = match FramePacketInformation::deserialize(&parts[0]) {
        Ok(info) => info,
        Err(e) => {
            tracing::warn!(
                "Dropping frame with undecodable header in aligned relay: {}",
                e
            );
            return None;
        }
    };
    let time = info.capture_time()?;
    Some(Sample {
        device_id: info.device_info.id,
        time,
        payload: parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{DeviceInformation, DeviceStatus, DeviceType};

    fn raw_frame(device: &str, tx_ms: u64, data: &[u8]) -> Vec<Vec<u8>> {
        let info = FramePacketInformation {
            device_info: DeviceInformation {
                id: device.to_string(),
                name: "test".to_string(),
                device_type: DeviceType::Camera,
                device_status: DeviceStatus::Available,
            },
            rx_timestamp: None,
            tx_timestamp: Some(SystemTime::UNIX_EPOCH + Duration::from_millis(tx_ms)),
            frame_shape: vec![2, 2],
            pixel_format: None,
            sample_rate: None,
            capture_timestamp: None,
        };
        vec![
            info.serialize().unwrap(),
            0u64.to_le_bytes().to_vec(),
            data.to_vec(),
        ]
    }

    fn group_of(samples: Vec<(&str, u64, &[u8])>, skew_ms: u64) -> AlignedGroup<Vec<Vec<u8>>> {
        AlignedGroup {
            samples: samples
                .into_iter()
                .map(|(device, tx_ms, data)| Sample {
                    device_id: device.to_string(),
                    time: SystemTime::UNIX_EPOCH + Duration::from_millis(tx_ms),
                    payload: raw_frame(device, tx_ms, data),
                })
                .collect(),
            skew: Duration::from_millis(skew_ms),
        }
    }

    #[test]
    fn group_round_trips_every_frame_and_the_skew() {
        let encoded = encode_group(group_of(
            vec![("cam", 1000, &[1, 2, 3]), ("mic", 1004, &[4, 5])],
            4,
        ));
        let decoded = decode_group(&encoded).unwrap();

        assert_eq!(decoded.skew, Duration::from_millis(4));
        assert_eq!(decoded.frames.len(), 2);
        assert_eq!(decoded.frames[0].frame_info.device_info.id, "cam");
        assert_eq!(&*decoded.frames[0].data, &[1u8, 2, 3][..]);
        assert_eq!(decoded.frames[1].frame_info.device_info.id, "mic");
        assert_eq!(&*decoded.frames[1].data, &[4u8, 5][..]);
        assert!(decoded.frames[0].frame_info.rx_timestamp.is_some());
    }

    #[test]
    fn decode_rejects_a_message_without_header() {
        assert!(decode_group(&[]).is_err());
    }

    #[test]
    fn decode_rejects_a_header_that_does_not_match_the_sample_count() {
        let mut encoded = encode_group(group_of(vec![("cam", 1000, &[1, 2, 3])], 0));
        encoded[0][8..12].copy_from_slice(&2u32.to_le_bytes());
        assert!(decode_group(&encoded).is_err());
    }

    #[test]
    fn decode_rejects_a_body_that_does_not_match_the_header() {
        let mut encoded = encode_group(group_of(vec![("cam", 1000, &[1, 2, 3])], 0));
        encoded.pop();
        assert!(decode_group(&encoded).is_err());
    }
}
