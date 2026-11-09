//! Loopback recordings: a synthetic YUYV camera and a microphone through a hub proxy, raw and
//! aligned. Skipped with a message when `ffmpeg` is not installed.

use std::collections::HashSet;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::{sleep, spawn};
use std::time::{Duration, SystemTime};

use networking::{AlignedRelay, Proxy, Sender};
use recorder::{Codec, RecorderConfig, Source, read_wav_header, record};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

const WIDTH: u32 = 16;
const HEIGHT: u32 = 8;
const SAMPLES: u32 = 512;
const RATE: u32 = 16_000;

fn camera_frame(id: &str, i: u8, captured: SystemTime) -> FramePacket {
    FramePacket::new(
        FramePacketInformation {
            device_info: DeviceInformation {
                id: id.into(),
                name: id.into(),
                device_type: DeviceType::Camera,
                device_status: DeviceStatus::Running,
            },
            rx_timestamp: None,
            tx_timestamp: None,
            frame_shape: vec![WIDTH, HEIGHT],
            pixel_format: Some("YUYV".into()),
            sample_rate: None,
            capture_timestamp: Some(captured),
        },
        vec![i; (WIDTH * HEIGHT * 2) as usize].into_boxed_slice(),
    )
}

fn mic_frame(captured: SystemTime) -> FramePacket {
    FramePacket::new(
        FramePacketInformation {
            device_info: DeviceInformation {
                id: "mic".into(),
                name: "mic".into(),
                device_type: DeviceType::Microphone,
                device_status: DeviceStatus::Running,
            },
            rx_timestamp: None,
            tx_timestamp: None,
            frame_shape: vec![SAMPLES, 1],
            pixel_format: None,
            sample_rate: Some(RATE),
            capture_timestamp: Some(captured),
        },
        vec![0u8; (SAMPLES * 2) as usize].into_boxed_slice(),
    )
}

fn ffmpeg_installed() -> bool {
    Command::new("ffprobe").arg("-version").output().is_ok()
        && Command::new("ffmpeg").arg("-version").output().is_ok()
}

fn counted_frames(path: &std::path::Path) -> u64 {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-count_frames",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=nb_read_frames",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .expect("ffprobe runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .trim_end_matches(',')
        .parse()
        .expect("frame count")
}

fn sidecar(path: &std::path::Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn micros(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("recorder-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn config(source: Source, port: u16, devices: &[&str], dir: &std::path::Path) -> RecorderConfig {
    RecorderConfig {
        hub_ip: std::net::Ipv4Addr::LOCALHOST,
        source,
        hub_port: port,
        devices: devices.iter().map(|d| d.to_string()).collect(),
        dir: dir.to_path_buf(),
        duration: Some(Duration::from_secs(3)),
        codec: Codec::Ffv1,
        calibration: None,
    }
}

/// One producer per device, as in a real deployment (each `device-node` numbers its own frames).
/// The hub fans all of them in. The proxy's own first producer endpoint points at an unused port.
fn producers(
    context: &zmq::Context,
    hub: &mut Proxy,
    devices: &[(&str, u16)],
) -> Vec<(String, Sender)> {
    devices
        .iter()
        .map(|(id, port)| {
            let endpoint = format!("tcp://127.0.0.1:{port}");
            hub.add_from_endpoint(endpoint.clone());
            let mut sender = Sender::new_endpoint(endpoint, 64, 1024);
            sender.start(context).unwrap();
            (id.to_string(), sender)
        })
        .collect()
}

/// Publishes 20 frames per device per second for `seconds` (`mic` sends microphone packets, the
/// others camera frames); returns the capture times it used.
fn publish(producers: &mut [(String, Sender)], seconds: u64) -> Vec<u64> {
    let mut times = Vec::new();
    for i in 0..(seconds * 20) {
        let captured = SystemTime::now();
        times.push(micros(captured));
        for (id, sender) in producers.iter_mut() {
            let frame = if id == "mic" {
                mic_frame(captured)
            } else {
                camera_frame(id, i as u8, captured)
            };
            sender.send(frame, zmq::DONTWAIT).unwrap();
        }
        sleep(Duration::from_millis(50));
    }
    times
}

#[test]
fn raw_recording_has_every_published_frame_and_sample() {
    if !ffmpeg_installed() {
        eprintln!("skipping: ffmpeg/ffprobe not installed");
        return;
    }
    let context = zmq::Context::new();
    let consumer_endpoint = "tcp://127.0.0.1:17302".to_string();
    let mut hub = Proxy::new_endpoints(
        "tcp://127.0.0.1:17399".to_string(),
        consumer_endpoint,
        64,
        1024,
    );
    let mut producers = producers(&context, &mut hub, &[("cam", 17301), ("mic", 17303)]);
    hub.start(&context).unwrap();

    let dir = temp_dir("raw");
    let cfg = config(Source::Raw, 17302, &["cam", "mic", "other"], &dir);
    let stop = Arc::new(AtomicBool::new(false));
    let recorder = {
        let stop = stop.clone();
        spawn(move || record(&cfg, &stop).unwrap())
    };
    sleep(Duration::from_millis(700));
    let published = publish(&mut producers, 2);
    let recording = recorder.join().unwrap();
    for (_, sender) in producers.iter_mut() {
        sender.stop().unwrap();
    }
    hub.stop().unwrap();

    let session: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(recording.dir.join("session.json")).unwrap())
            .unwrap();
    assert_eq!(session["source"], "raw");
    assert!(session["stopped"].is_string());
    let devices = session["devices"].as_array().unwrap();
    assert_eq!(devices.len(), 3);
    let cam = &devices[0];
    assert_eq!(cam["device_type"], "Camera");
    assert_eq!(cam["codec"], "ffv1");
    assert_eq!(cam["pixel_format"], "YUYV");
    assert_eq!(cam["frames_dropped"], 0);
    assert_eq!(cam["sequence_gaps"], 0);
    let written = cam["frames_written"].as_u64().unwrap();
    assert_eq!(
        written,
        published.len() as u64,
        "every camera frame is written"
    );

    let lines = sidecar(&recording.dir.join("cam.jsonl"));
    assert_eq!(lines.len() as u64, written);
    assert_eq!(counted_frames(&recording.dir.join("cam.mkv")), written);
    let recorded: Vec<u64> = lines
        .iter()
        .map(|l| l["capture_us"].as_u64().unwrap())
        .collect();
    assert_eq!(
        recorded, published,
        "sidecar capture times equal the published ones"
    );
    assert_eq!(lines[0]["index"], 0);
    assert_eq!(lines[0]["width"], WIDTH);
    assert!(
        lines[0]["sequence"].is_u64(),
        "wire sequence recorded: {}",
        lines[0]
    );
    assert_eq!(
        lines[1]["sequence"].as_u64().unwrap(),
        lines[0]["sequence"].as_u64().unwrap() + 1
    );

    let mic = &devices[1];
    assert_eq!(mic["device_type"], "Microphone");
    assert_eq!(mic["codec"], "wav");
    let packets = mic["frames_written"].as_u64().unwrap();
    assert_eq!(packets, published.len() as u64);
    let wav = std::fs::read(recording.dir.join("mic.wav")).unwrap();
    let (rate, channels, data_bytes) = read_wav_header(&wav).unwrap();
    assert_eq!((rate, channels), (RATE, 1));
    assert_eq!(u64::from(data_bytes), packets * u64::from(SAMPLES) * 2);
    assert_eq!(wav.len() as u64, 44 + u64::from(data_bytes));
    let mic_lines = sidecar(&recording.dir.join("mic.jsonl"));
    assert_eq!(mic_lines[0]["samples"], SAMPLES);
    assert!(mic_lines[0].get("width").is_none());

    let other = &devices[2];
    assert_eq!(
        other["device_type"], "unknown",
        "a device that never sent stays unknown"
    );
    assert_eq!(other["frames_written"], 0);
    assert!(!recording.dir.join("other.mkv").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn aligned_recording_writes_only_complete_groups() {
    if !ffmpeg_installed() {
        eprintln!("skipping: ffmpeg/ffprobe not installed");
        return;
    }
    let context = zmq::Context::new();
    let consumer_endpoint = "tcp://127.0.0.1:17312".to_string();
    let sync_endpoint = "tcp://127.0.0.1:17313".to_string();
    let mut hub = Proxy::new_endpoints(
        "tcp://127.0.0.1:17398".to_string(),
        consumer_endpoint.clone(),
        64,
        1024,
    );
    let mut producers = producers(
        &context,
        &mut hub,
        &[("cam-a", 17311), ("cam-b", 17314), ("mic", 17315)],
    );
    hub.start(&context).unwrap();
    let mut relay = AlignedRelay::new(
        vec![consumer_endpoint],
        sync_endpoint,
        vec!["cam-a".into(), "cam-b".into(), "mic".into()],
        Duration::from_millis(20),
    );
    relay.start(&context).unwrap();

    let dir = temp_dir("aligned");
    let cfg = config(Source::Aligned, 17313, &["cam-a", "cam-b"], &dir);
    let stop = Arc::new(AtomicBool::new(false));
    let recorder = {
        let stop = stop.clone();
        spawn(move || record(&cfg, &stop).unwrap())
    };
    sleep(Duration::from_millis(700));
    // One lone frame from cam-a: its group can never complete and must not be written.
    producers[0]
        .1
        .send(camera_frame("cam-a", 99, SystemTime::now()), zmq::DONTWAIT)
        .unwrap();
    sleep(Duration::from_millis(300));
    let published = publish(&mut producers, 2);
    let recording = recorder.join().unwrap();
    for (_, sender) in producers.iter_mut() {
        sender.stop().unwrap();
    }
    relay.stop().unwrap();
    hub.stop().unwrap();

    assert_eq!(recording.info.source, "aligned");
    assert_eq!(recording.info.devices.len(), 2);
    let a = &recording.info.devices[0];
    let b = &recording.info.devices[1];
    assert_eq!(
        a.frames_written, b.frames_written,
        "same frame count per file"
    );
    assert!(
        a.frames_written >= published.len() as u64 - 1,
        "{} written",
        a.frames_written
    );
    assert!(a.frames_written <= published.len() as u64);
    assert_eq!(
        counted_frames(&recording.dir.join("cam-a.mkv")),
        a.frames_written
    );
    assert_eq!(
        counted_frames(&recording.dir.join("cam-b.mkv")),
        b.frames_written
    );

    let lines_a = sidecar(&recording.dir.join("cam-a.jsonl"));
    let lines_b = sidecar(&recording.dir.join("cam-b.jsonl"));
    let times_a: Vec<u64> = lines_a
        .iter()
        .map(|l| l["capture_us"].as_u64().unwrap())
        .collect();
    let times_b: Vec<u64> = lines_b
        .iter()
        .map(|l| l["capture_us"].as_u64().unwrap())
        .collect();
    assert_eq!(times_a, times_b, "files line up row by row");
    let published: HashSet<u64> = published.into_iter().collect();
    assert!(
        times_a.iter().all(|t| published.contains(t)),
        "the lone frame was skipped"
    );
    assert!(
        !recording.dir.join("mic.wav").exists(),
        "mic was in the groups but not recorded"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
