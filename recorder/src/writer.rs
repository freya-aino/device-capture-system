//! One writer thread per recorded device, behind a bounded queue. The main thread never blocks on
//! disk or `ffmpeg`: when a queue is full the frame is dropped and counted.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::thread::JoinHandle;

use anyhow::{Context, Result, anyhow, bail, ensure};
use shared::{DeviceType, FramePacket};

use crate::ffmpeg::{FfmpegWriter, codec_name, ffmpeg_args};
use crate::wav::WavWriter;
use crate::{Codec, DeviceRecord, SidecarLine};

/// Frames a writer may have waiting before the main thread starts dropping for it.
pub const QUEUE_LEN: usize = 64;
/// Frames whose capture times set the nominal rate passed to `ffmpeg`.
pub const RATE_SAMPLE_FRAMES: usize = 30;
/// Rate used when fewer than two timestamped frames were seen.
pub const FALLBACK_FPS: f64 = 30.0;

struct Job {
    packet: FramePacket,
    sequence: Option<u64>,
}

/// What the writer thread reports when it ends.
struct Outcome {
    frames_written: u64,
    device_type: Option<String>,
    pixel_format: Option<String>,
    frame_shape: Vec<u32>,
    codec: Option<String>,
    error: Option<String>,
}

/// Main-thread handle to one device's writer thread, and the counters the main thread keeps.
pub struct DeviceWriter {
    tx: Option<SyncSender<Job>>,
    thread: Option<JoinHandle<Outcome>>,
    record: DeviceRecord,
    last_sequence: Option<u64>,
}

impl DeviceWriter {
    pub fn start(device_id: &str, file_stem: &str, dir: &Path, codec: Codec) -> Result<Self> {
        let sidecar_path = dir.join(format!("{file_stem}.jsonl"));
        let sidecar = File::create(&sidecar_path)
            .with_context(|| format!("creating {}", sidecar_path.display()))?;
        let (tx, rx) = sync_channel(QUEUE_LEN);
        let thread = WriterThread {
            dir: dir.to_path_buf(),
            file_stem: file_stem.to_string(),
            codec,
            sidecar: BufWriter::new(sidecar),
            sink: Sink::Unset,
            pending: Vec::new(),
            written: 0,
        };
        let handle = std::thread::Builder::new()
            .name(format!("record-{file_stem}"))
            .spawn(move || thread.run(rx))?;
        Ok(DeviceWriter {
            tx: Some(tx),
            thread: Some(handle),
            record: DeviceRecord {
                device_id: device_id.to_string(),
                file_stem: file_stem.to_string(),
                device_type: "unknown".to_string(),
                pixel_format: None,
                frame_shape: Vec::new(),
                codec: "unknown".to_string(),
                frames_written: 0,
                frames_dropped: 0,
                sequence_gaps: 0,
                error: None,
            },
            last_sequence: None,
        })
    }

    /// The record as known on the main thread (drops and gaps are live; the writer's counts arrive
    /// with `finish`).
    pub fn record(&self) -> &DeviceRecord {
        &self.record
    }

    /// Hands a frame to the writer thread, or counts it as dropped when the queue is full.
    pub fn push(&mut self, packet: FramePacket, sequence: Option<u64>) {
        if let Some(sequence) = sequence {
            if let Some(last) = self.last_sequence {
                if sequence > last + 1 {
                    self.record.sequence_gaps += sequence - last - 1;
                }
            }
            self.last_sequence = Some(sequence);
        }
        let Some(tx) = &self.tx else { return };
        match tx.try_send(Job { packet, sequence }) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => self.record.frames_dropped += 1,
            // The writer stopped with an error; it is reported by `finish`.
            Err(TrySendError::Disconnected(_)) => {}
        }
    }

    /// Closes the queue, waits for the writer to finish its files and returns the final record.
    pub fn finish(mut self) -> DeviceRecord {
        drop(self.tx.take());
        let mut record = self.record;
        if let Some(handle) = self.thread.take() {
            match handle.join() {
                Ok(outcome) => {
                    record.frames_written = outcome.frames_written;
                    if let Some(t) = outcome.device_type {
                        record.device_type = t;
                    }
                    record.pixel_format = outcome.pixel_format;
                    record.frame_shape = outcome.frame_shape;
                    if let Some(c) = outcome.codec {
                        record.codec = c;
                    }
                    record.error = outcome.error;
                }
                Err(_) => record.error = Some("writer thread panicked".to_string()),
            }
        }
        record
    }
}

enum Sink {
    Unset,
    /// Camera frames held until the nominal rate can be measured and `ffmpeg` started.
    PendingVideo,
    Video(FfmpegWriter),
    Wav(WavWriter),
}

struct WriterThread {
    dir: PathBuf,
    file_stem: String,
    codec: Codec,
    sidecar: BufWriter<File>,
    sink: Sink,
    pending: Vec<Job>,
    written: u64,
}

impl WriterThread {
    fn run(mut self, rx: Receiver<Job>) -> Outcome {
        let mut outcome = Outcome {
            frames_written: 0,
            device_type: None,
            pixel_format: None,
            frame_shape: Vec::new(),
            codec: None,
            error: None,
        };
        for job in rx {
            if let Err(e) = self.handle(job, &mut outcome) {
                outcome.error = Some(format!("{e:#}"));
                break;
            }
        }
        if let Err(e) = self.close(&mut outcome) {
            if outcome.error.is_none() {
                outcome.error = Some(format!("{e:#}"));
            }
        }
        outcome.frames_written = self.written;
        outcome
    }

    fn handle(&mut self, job: Job, outcome: &mut Outcome) -> Result<()> {
        let info = &job.packet.frame_info;
        let device_type = match info.device_info.device_type {
            DeviceType::Camera => "Camera",
            DeviceType::Microphone => "Microphone",
        };
        match &outcome.device_type {
            None => {
                outcome.device_type = Some(device_type.to_string());
                outcome.pixel_format = info.pixel_format.clone();
                outcome.frame_shape = info.frame_shape.clone();
            }
            Some(first) => {
                ensure!(
                    first == device_type,
                    "device type changed from {first} to {device_type}"
                );
                ensure!(
                    outcome.frame_shape == info.frame_shape,
                    "frame_shape changed from {:?} to {:?} mid-session",
                    outcome.frame_shape,
                    info.frame_shape
                );
                ensure!(
                    outcome.pixel_format == info.pixel_format,
                    "pixel_format changed from {:?} to {:?} mid-session",
                    outcome.pixel_format,
                    info.pixel_format
                );
            }
        }

        match info.device_info.device_type {
            DeviceType::Camera => {
                if matches!(self.sink, Sink::Unset) {
                    let format = info
                        .pixel_format
                        .as_deref()
                        .ok_or_else(|| anyhow!("camera frame has no pixel_format"))?;
                    outcome.codec = Some(codec_name(format, self.codec).to_string());
                    self.sink = Sink::PendingVideo;
                }
                match &mut self.sink {
                    Sink::PendingVideo => {
                        self.pending.push(job);
                        if self.pending.len() >= RATE_SAMPLE_FRAMES {
                            self.start_video()?;
                        }
                    }
                    Sink::Video(_) => self.write_frame(job)?,
                    _ => unreachable!("camera sink is video"),
                }
            }
            DeviceType::Microphone => {
                if matches!(self.sink, Sink::Unset) {
                    let rate = info
                        .sample_rate
                        .ok_or_else(|| anyhow!("microphone frame has no sample_rate"))?;
                    let channels = info.frame_shape.get(1).copied().unwrap_or(1);
                    let channels = u16::try_from(channels)
                        .map_err(|_| anyhow!("channel count {channels} is out of range"))?;
                    let path = self.dir.join(format!("{}.wav", self.file_stem));
                    self.sink = Sink::Wav(WavWriter::create(&path, rate, channels)?);
                    outcome.codec = Some("wav".to_string());
                }
                self.write_frame(job)?;
            }
        }
        Ok(())
    }

    /// Starts `ffmpeg` with the rate measured from the pending frames, then writes them.
    fn start_video(&mut self) -> Result<()> {
        let first = self
            .pending
            .first()
            .ok_or_else(|| anyhow!("no frames to start the video with"))?;
        let info = &first.packet.frame_info;
        let format = info.pixel_format.clone().unwrap_or_default();
        let (width, height) = (
            info.frame_shape.first().copied().unwrap_or(0),
            info.frame_shape.get(1).copied().unwrap_or(0),
        );
        ensure!(
            width > 0 && height > 0,
            "camera frame_shape {:?} is empty",
            info.frame_shape
        );
        let fps = measured_fps(self.pending.iter().map(|j| &j.packet));
        let path = self.dir.join(format!("{}.mkv", self.file_stem));
        let args = ffmpeg_args(&format, width, height, fps, self.codec, &path)?;
        tracing::info!(
            "{}: starting ffmpeg for {format} {width}x{height} at {fps:.3} fps -> {}",
            self.file_stem,
            path.display()
        );
        self.sink = Sink::Video(FfmpegWriter::spawn(&args, &path)?);
        for job in std::mem::take(&mut self.pending) {
            self.write_frame(job)?;
        }
        Ok(())
    }

    fn write_frame(&mut self, job: Job) -> Result<()> {
        match &mut self.sink {
            Sink::Video(ffmpeg) => ffmpeg.write_frame(&job.packet.data)?,
            Sink::Wav(wav) => wav.write(&job.packet.data)?,
            Sink::Unset | Sink::PendingVideo => bail!("writer has no open sink"),
        }
        let line = SidecarLine::from_packet(self.written, job.sequence, &job.packet);
        self.sidecar.write_all(line.to_json().as_bytes())?;
        self.sidecar.write_all(b"\n")?;
        self.written += 1;
        Ok(())
    }

    fn close(&mut self, _outcome: &mut Outcome) -> Result<()> {
        if matches!(self.sink, Sink::PendingVideo) && !self.pending.is_empty() {
            self.start_video()?;
        }
        match std::mem::replace(&mut self.sink, Sink::Unset) {
            Sink::Video(ffmpeg) => ffmpeg.finish()?,
            Sink::Wav(wav) => wav.finish()?,
            Sink::Unset | Sink::PendingVideo => {}
        }
        self.sidecar.flush()?;
        Ok(())
    }
}

/// Frames per second from the capture times of a run of frames: `(n - 1) / span`, or
/// `FALLBACK_FPS` when fewer than two timestamped frames are available.
pub fn measured_fps<'a>(frames: impl Iterator<Item = &'a FramePacket>) -> f64 {
    let times: Vec<_> = frames.filter_map(|p| p.frame_info.capture_time()).collect();
    let (Some(first), Some(last)) = (times.first(), times.last()) else {
        return FALLBACK_FPS;
    };
    match last.duration_since(*first) {
        Ok(span) if span.as_secs_f64() > 0.0 && times.len() >= 2 => {
            (times.len() - 1) as f64 / span.as_secs_f64()
        }
        _ => FALLBACK_FPS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{DeviceInformation, DeviceStatus, FramePacketInformation};
    use std::time::{Duration, SystemTime};

    fn frame_at(ms: u64) -> FramePacket {
        FramePacket::new(
            FramePacketInformation {
                device_info: DeviceInformation {
                    id: "cam".into(),
                    name: "cam".into(),
                    device_type: DeviceType::Camera,
                    device_status: DeviceStatus::Running,
                },
                rx_timestamp: None,
                tx_timestamp: None,
                frame_shape: vec![2, 2],
                pixel_format: Some("YUYV".into()),
                sample_rate: None,
                capture_timestamp: Some(SystemTime::UNIX_EPOCH + Duration::from_millis(ms)),
            },
            Box::new([]),
        )
    }

    #[test]
    fn fps_is_measured_from_capture_times() {
        let frames: Vec<_> = (0..30).map(|i| frame_at(1000 + i * 40)).collect();
        assert!((measured_fps(frames.iter()) - 25.0).abs() < 1e-9);
        assert_eq!(measured_fps(frames.iter().take(1)), FALLBACK_FPS);
        assert_eq!(
            measured_fps([frame_at(5), frame_at(5)].iter()),
            FALLBACK_FPS
        );
    }
}
