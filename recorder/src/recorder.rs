//! The recording session: writers per device, the hub input, and `session.json`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result, bail};

use crate::ffmpeg::ffmpeg_available;
use crate::input::Input;
use crate::writer::DeviceWriter;
use crate::{RecorderConfig, SessionInfo, StemAllocator, rfc3339, session_dir};

/// The git revision of this build, from `build.rs`.
pub const REVISION: &str = env!("RECORDER_REVISION");

/// Where a finished session is and what it recorded.
pub struct Recording {
    pub dir: PathBuf,
    pub info: SessionInfo,
}

/// Records until `stop` is set or `config.duration` elapses, then finishes every file and
/// rewrites `session.json`. Returns the session directory and its final `session.json` content.
pub fn record(config: &RecorderConfig, stop: &AtomicBool) -> Result<Recording> {
    if !ffmpeg_available() {
        bail!(
            "ffmpeg is not on the PATH; the recorder needs it to write camera streams (it cannot \
             know a device's type before frames arrive, so it is required at start)"
        );
    }

    let started = SystemTime::now();
    let dir = session_dir(&config.dir, started);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    if let Some(calibration) = &config.calibration {
        std::fs::copy(calibration, dir.join("calibration.json")).with_context(|| {
            format!(
                "copying RECORD_CALIBRATION {} into the session",
                calibration.display()
            )
        })?;
    }

    let mut stems = StemAllocator::default();
    let mut writers: HashMap<String, DeviceWriter> = HashMap::new();
    for device in &config.devices {
        if writers.contains_key(device) {
            continue;
        }
        let stem = stems.stem(device);
        writers.insert(
            device.clone(),
            DeviceWriter::start(device, &stem, &dir, config.codec)?,
        );
    }

    let session = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut info = SessionInfo {
        session,
        revision: REVISION.to_string(),
        source: config.source.name().to_string(),
        hub: config.hub_endpoint(),
        started: rfc3339(started),
        stopped: None,
        incomplete_groups: None,
        devices: config
            .devices
            .iter()
            .filter_map(|d| writers.get(d).map(|w| w.record().clone()))
            .collect(),
    };
    write_session(&dir, &info)?;

    let mut input = Input::connect(config)?;
    tracing::info!(
        "recording {} from {} into {}",
        config.devices.join(", "),
        info.hub,
        dir.display()
    );

    let begin = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        if let Some(duration) = config.duration {
            if begin.elapsed() >= duration {
                break;
            }
        }
        for received in input.next(Duration::from_millis(100))? {
            if let Some(writer) = writers.get_mut(&received.packet.frame_info.device_info.id) {
                writer.push(received.packet, received.sequence);
            }
        }
    }

    if let Err(e) = input.disconnect() {
        tracing::warn!("disconnecting from the hub: {e:#}");
    }
    let incomplete = input.incomplete_groups();
    if incomplete > 0 {
        tracing::warn!("{incomplete} aligned groups lacked a recorded device and were skipped");
        info.incomplete_groups = Some(incomplete);
    }

    info.devices = config
        .devices
        .iter()
        .filter_map(|d| writers.remove(d))
        .map(DeviceWriter::finish)
        .collect();
    info.stopped = Some(rfc3339(SystemTime::now()));
    write_session(&dir, &info)?;
    for device in &info.devices {
        match &device.error {
            Some(error) => tracing::error!(
                "{}: {} frames written, then stopped: {error}",
                device.device_id,
                device.frames_written
            ),
            None => tracing::info!(
                "{}: {} frames written, {} dropped (writer backlog), {} sequence gaps",
                device.device_id,
                device.frames_written,
                device.frames_dropped,
                device.sequence_gaps
            ),
        }
    }
    Ok(Recording { dir, info })
}

fn write_session(dir: &std::path::Path, info: &SessionInfo) -> Result<()> {
    let path = dir.join("session.json");
    std::fs::write(&path, info.to_json()).with_context(|| format!("writing {}", path.display()))
}
