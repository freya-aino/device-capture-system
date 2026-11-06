//! The on-disk formats: one sidecar line per written frame, and `session.json`.

use serde::Serialize;
use shared::{DeviceType, FramePacket};

use crate::utc::micros_since_epoch;

/// One line of `<file-stem>.jsonl`. Cameras carry `width`/`height`, microphones `samples`/`channels`;
/// the other pair is omitted.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SidecarLine {
    /// Position of the frame in the video, or of the packet in the WAV.
    pub index: u64,
    /// Wire sequence number, when the receiver exposes it.
    pub sequence: Option<u64>,
    pub capture_us: Option<u64>,
    pub tx_us: Option<u64>,
    pub rx_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_format: Option<String>,
}

impl SidecarLine {
    pub fn from_packet(index: u64, sequence: Option<u64>, packet: &FramePacket) -> Self {
        let info = &packet.frame_info;
        let dim = |i: usize| info.frame_shape.get(i).copied();
        let camera = info.device_info.device_type == DeviceType::Camera;
        SidecarLine {
            index,
            sequence,
            capture_us: info.capture_timestamp.and_then(micros_since_epoch),
            tx_us: info.tx_timestamp.and_then(micros_since_epoch),
            rx_us: info.rx_timestamp.and_then(micros_since_epoch),
            width: if camera { dim(0) } else { None },
            height: if camera { dim(1) } else { None },
            samples: if camera { None } else { dim(0) },
            channels: if camera { None } else { dim(1) },
            pixel_format: info.pixel_format.clone(),
        }
    }

    /// The line as written, without the trailing newline.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("sidecar lines always serialize")
    }
}

/// One recorded device in `session.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeviceRecord {
    pub device_id: String,
    pub file_stem: String,
    pub device_type: String,
    pub pixel_format: Option<String>,
    pub frame_shape: Vec<u32>,
    /// `copy`, `ffv1`, `h264` or `wav`.
    pub codec: String,
    pub frames_written: u64,
    pub frames_dropped: u64,
    pub sequence_gaps: u64,
    /// Why the device's writer stopped early, if it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// `session.json`: written at start with `stopped: null`, rewritten at stop.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionInfo {
    pub session: String,
    pub revision: String,
    pub source: String,
    pub hub: String,
    pub started: String,
    pub stopped: Option<String>,
    pub devices: Vec<DeviceRecord>,
}

impl SessionInfo {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("session info always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{DeviceInformation, DeviceStatus, FramePacketInformation};
    use std::time::{Duration, SystemTime};

    fn packet(device_type: DeviceType, shape: Vec<u32>, format: Option<&str>) -> FramePacket {
        let at = |us| SystemTime::UNIX_EPOCH + Duration::from_micros(us);
        FramePacket::new(
            FramePacketInformation {
                device_info: DeviceInformation {
                    id: "dev".into(),
                    name: "n".into(),
                    device_type,
                    device_status: DeviceStatus::Running,
                },
                rx_timestamp: Some(at(1_000_300)),
                tx_timestamp: Some(at(1_000_200)),
                frame_shape: shape,
                pixel_format: format.map(str::to_string),
                sample_rate: None,
                capture_timestamp: Some(at(1_000_100)),
            },
            Box::new([]),
        )
    }

    #[test]
    fn camera_sidecar_line_matches_the_spec() {
        let line = SidecarLine::from_packet(
            0,
            Some(1234),
            &packet(DeviceType::Camera, vec![1920, 1080], Some("MJPG")),
        );
        assert_eq!(
            line.to_json(),
            r#"{"index":0,"sequence":1234,"capture_us":1000100,"tx_us":1000200,"rx_us":1000300,"width":1920,"height":1080,"pixel_format":"MJPG"}"#
        );
    }

    #[test]
    fn microphone_sidecar_line_has_samples_and_channels() {
        let line =
            SidecarLine::from_packet(7, None, &packet(DeviceType::Microphone, vec![512, 1], None));
        assert_eq!(
            line.to_json(),
            r#"{"index":7,"sequence":null,"capture_us":1000100,"tx_us":1000200,"rx_us":1000300,"samples":512,"channels":1}"#
        );
    }

    #[test]
    fn session_info_serializes_with_the_spec_field_names() {
        let info = SessionInfo {
            session: "20261106T101500Z".into(),
            revision: "test".into(),
            source: "aligned".into(),
            hub: "tcp://127.0.0.1:5557".into(),
            started: "2026-11-06T10:15:00.000000Z".into(),
            stopped: None,
            devices: vec![DeviceRecord {
                device_id: "/dev/video0".into(),
                file_stem: "dev_video0".into(),
                device_type: "Camera".into(),
                pixel_format: Some("MJPG".into()),
                frame_shape: vec![1920, 1080],
                codec: "copy".into(),
                frames_written: 0,
                frames_dropped: 0,
                sequence_gaps: 0,
                error: None,
            }],
        };
        let value: serde_json::Value = serde_json::from_str(&info.to_json()).unwrap();
        assert_eq!(value["stopped"], serde_json::Value::Null);
        assert_eq!(value["devices"][0]["file_stem"], "dev_video0");
        assert!(value["devices"][0].get("error").is_none());
    }
}
