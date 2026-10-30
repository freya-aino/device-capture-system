use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use networking::FrameGroup;
use shared::FramePacket;

/// Per-device counters over one reporting window: how many frames arrived, how many bytes, and
/// the per-frame latency (`rx_timestamp - tx_timestamp`) when both stamps are present.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeviceWindow {
    pub frames: u64,
    pub bytes: u64,
    pub latency_sum: Duration,
    pub latency_max: Duration,
    pub latency_samples: u64,
    /// `rx_timestamp - capture_time`: how old a frame is when it reaches the consumer, including
    /// device-side buffering (a microphone packet waits for its quantum to fill).
    pub age_sum: Duration,
    pub age_max: Duration,
    pub age_samples: u64,
    pub frame_shape: Vec<u32>,
    pub pixel_format: Option<String>,
}

impl DeviceWindow {
    fn record(&mut self, packet: &FramePacket) {
        self.frames += 1;
        self.bytes += packet.data.len() as u64;
        self.frame_shape = packet.frame_info.frame_shape.clone();
        self.pixel_format = packet.frame_info.pixel_format.clone();
        if let (Some(tx), Some(rx)) = (
            packet.frame_info.tx_timestamp,
            packet.frame_info.rx_timestamp,
        ) && let Ok(latency) = rx.duration_since(tx)
        {
            self.latency_sum += latency;
            self.latency_max = self.latency_max.max(latency);
            self.latency_samples += 1;
        }
        if let (Some(captured), Some(rx)) = (
            packet.frame_info.capture_time(),
            packet.frame_info.rx_timestamp,
        ) && let Ok(age) = rx.duration_since(captured)
        {
            self.age_sum += age;
            self.age_max = self.age_max.max(age);
            self.age_samples += 1;
        }
    }

    pub fn mean_age(&self) -> Option<Duration> {
        (self.age_samples > 0).then(|| self.age_sum / self.age_samples as u32)
    }

    pub fn mean_latency(&self) -> Option<Duration> {
        (self.latency_samples > 0).then(|| self.latency_sum / self.latency_samples as u32)
    }
}

/// Accumulates frames per device id and renders one summary line per device when the window is
/// over. The `access-client` CLI prints these when `CLIENT_SUMMARY_SECS` is set; the hardware
/// validation run (`docs/specs` step 12) reads its numbers from them.
#[derive(Debug)]
pub struct FrameSummary {
    window: Duration,
    window_started: Instant,
    devices: BTreeMap<String, DeviceWindow>,
    totals: BTreeMap<String, u64>,
}

impl FrameSummary {
    pub fn new(window: Duration) -> Self {
        FrameSummary {
            window,
            window_started: Instant::now(),
            devices: BTreeMap::new(),
            totals: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, packet: &FramePacket) {
        let id = packet.frame_info.device_info.id.clone();
        self.devices.entry(id.clone()).or_default().record(packet);
        *self.totals.entry(id).or_insert(0) += 1;
    }

    pub fn window_elapsed(&self) -> bool {
        self.window_started.elapsed() >= self.window
    }

    /// Finishes the current window: returns one line per device seen in it and starts a new one.
    /// `dropped_total` is the receiver's running dropped-frame count, printed with every line
    /// because the receiver does not attribute drops to a device.
    pub fn flush(&mut self, dropped_total: u64) -> Vec<String> {
        let elapsed = self.window_started.elapsed();
        let lines = self
            .devices
            .iter()
            .map(|(id, w)| render_line(id, w, elapsed, self.totals[id], dropped_total))
            .collect();
        self.devices.clear();
        self.window_started = Instant::now();
        lines
    }

    pub fn totals(&self) -> &BTreeMap<String, u64> {
        &self.totals
    }
}

fn render_line(id: &str, w: &DeviceWindow, elapsed: Duration, total: u64, dropped: u64) -> String {
    let secs = elapsed.as_secs_f64().max(f64::EPSILON);
    let latency = match (w.mean_latency(), w.latency_samples) {
        (Some(mean), _) => format!(
            "latency_ms mean={:.1} max={:.1}",
            mean.as_secs_f64() * 1000.0,
            w.latency_max.as_secs_f64() * 1000.0
        ),
        _ => "latency_ms n/a".to_string(),
    };
    let age = match w.mean_age() {
        Some(mean) => format!(
            "age_ms mean={:.1} max={:.1}",
            mean.as_secs_f64() * 1000.0,
            w.age_max.as_secs_f64() * 1000.0
        ),
        None => "age_ms n/a".to_string(),
    };
    format!(
        "summary: device={id} window_s={secs:.1} frames={} rate={:.1}/s mbit/s={:.1} {latency} {age} shape={:?} format={} total_frames={total} dropped_total={dropped}",
        w.frames,
        w.frames as f64 / secs,
        w.bytes as f64 * 8.0 / 1e6 / secs,
        w.frame_shape,
        w.pixel_format.as_deref().unwrap_or("-"),
    )
}

/// Accumulates aligned groups from a `GroupReceiver` (the hub's `HUB_SYNC_PORT`) over a reporting
/// window: how many groups arrived, the skew inside them, and how many frames each device
/// contributed. Printed by the `access-client` CLI when `CLIENT_ALIGNED_PORT` is set.
#[derive(Debug)]
pub struct GroupSummary {
    window: Duration,
    window_started: Instant,
    groups: u64,
    skew_sum: Duration,
    skew_max: Duration,
    latency: DeviceWindow,
    per_device: BTreeMap<String, u64>,
    total_groups: u64,
}

impl GroupSummary {
    pub fn new(window: Duration) -> Self {
        GroupSummary {
            window,
            window_started: Instant::now(),
            groups: 0,
            skew_sum: Duration::ZERO,
            skew_max: Duration::ZERO,
            latency: DeviceWindow::default(),
            per_device: BTreeMap::new(),
            total_groups: 0,
        }
    }

    pub fn record(&mut self, group: &FrameGroup) {
        self.groups += 1;
        self.total_groups += 1;
        self.skew_sum += group.skew;
        self.skew_max = self.skew_max.max(group.skew);
        for frame in &group.frames {
            self.latency.record(frame);
            *self
                .per_device
                .entry(frame.frame_info.device_info.id.clone())
                .or_insert(0) += 1;
        }
    }

    pub fn window_elapsed(&self) -> bool {
        self.window_started.elapsed() >= self.window
    }

    pub fn total_groups(&self) -> u64 {
        self.total_groups
    }

    /// Finishes the window and returns its summary line (`None` when no group arrived).
    pub fn flush(&mut self) -> Option<String> {
        let elapsed = self.window_started.elapsed();
        let line = (self.groups > 0).then(|| {
            let secs = elapsed.as_secs_f64().max(f64::EPSILON);
            let devices: Vec<String> = self
                .per_device
                .iter()
                .map(|(id, n)| format!("{id}={n}"))
                .collect();
            let latency = match self.latency.mean_latency() {
                Some(mean) => format!(
                    "latency_ms mean={:.1} max={:.1}",
                    mean.as_secs_f64() * 1000.0,
                    self.latency.latency_max.as_secs_f64() * 1000.0
                ),
                None => "latency_ms n/a".to_string(),
            };
            format!(
                "aligned: window_s={secs:.1} groups={} rate={:.1}/s skew_ms mean={:.1} max={:.1} {latency} frames=[{}] total_groups={}",
                self.groups,
                self.groups as f64 / secs,
                (self.skew_sum / self.groups as u32).as_secs_f64() * 1000.0,
                self.skew_max.as_secs_f64() * 1000.0,
                devices.join(" "),
                self.total_groups
            )
        });
        self.groups = 0;
        self.skew_sum = Duration::ZERO;
        self.skew_max = Duration::ZERO;
        self.latency = DeviceWindow::default();
        self.per_device.clear();
        self.window_started = Instant::now();
        line
    }
}

/// Convenience for callers that only want the latency of one packet.
pub fn packet_latency(packet: &FramePacket) -> Option<Duration> {
    match (
        packet.frame_info.tx_timestamp,
        packet.frame_info.rx_timestamp,
    ) {
        (Some(tx), Some(rx)) => rx.duration_since(tx).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacketInformation};
    use std::time::UNIX_EPOCH;

    fn packet(id: &str, tx_ms: u64, rx_ms: u64, bytes: usize) -> FramePacket {
        FramePacket::new(
            FramePacketInformation {
                device_info: DeviceInformation {
                    id: id.to_string(),
                    name: id.to_string(),
                    device_type: DeviceType::Camera,
                    device_status: DeviceStatus::Running,
                },
                rx_timestamp: Some(UNIX_EPOCH + Duration::from_millis(rx_ms)),
                tx_timestamp: Some(UNIX_EPOCH + Duration::from_millis(tx_ms)),
                frame_shape: vec![4, 2],
                pixel_format: Some("YUYV".to_string()),
                sample_rate: None,
                capture_timestamp: None,
            },
            vec![0u8; bytes].into_boxed_slice(),
        )
    }

    #[test]
    fn device_window_tracks_frames_bytes_and_latency() {
        let mut w = DeviceWindow::default();
        w.record(&packet("cam", 1000, 1010, 16));
        w.record(&packet("cam", 1033, 1063, 16));
        assert_eq!(w.frames, 2);
        assert_eq!(w.bytes, 32);
        assert_eq!(w.mean_latency(), Some(Duration::from_millis(20)));
        assert_eq!(w.latency_max, Duration::from_millis(30));
        assert_eq!(w.pixel_format.as_deref(), Some("YUYV"));
        assert_eq!(
            w.mean_age(),
            Some(Duration::from_millis(20)),
            "age falls back to tx"
        );

        let mut buffered = packet("mic", 1100, 1105, 16);
        buffered.frame_info.capture_timestamp = Some(UNIX_EPOCH + Duration::from_millis(1000));
        let mut w = DeviceWindow::default();
        w.record(&buffered);
        assert_eq!(w.mean_latency(), Some(Duration::from_millis(5)));
        assert_eq!(w.mean_age(), Some(Duration::from_millis(105)));
    }

    #[test]
    fn latency_is_skipped_when_a_stamp_is_missing_or_negative() {
        let mut w = DeviceWindow::default();
        let mut no_rx = packet("cam", 1000, 1010, 1);
        no_rx.frame_info.rx_timestamp = None;
        w.record(&no_rx);
        w.record(&packet("cam", 2000, 1990, 1)); // rx before tx: clock skew, ignored
        assert_eq!(w.frames, 2);
        assert_eq!(w.mean_latency(), None);
    }

    #[test]
    fn summary_flush_renders_one_line_per_device_and_resets_the_window() {
        let mut s = FrameSummary::new(Duration::from_secs(10));
        s.record(&packet("cam", 0, 5, 1000));
        s.record(&packet("cam", 33, 40, 1000));
        s.record(&packet("mic", 0, 1, 1024));
        let lines = s.flush(3);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("summary: device=cam "), "{}", lines[0]);
        assert!(lines[0].contains("frames=2"), "{}", lines[0]);
        assert!(
            lines[0].contains("latency_ms mean=6.0 max=7.0"),
            "{}",
            lines[0]
        );
        assert!(lines[0].contains("format=YUYV"), "{}", lines[0]);
        assert!(lines[0].contains("dropped_total=3"), "{}", lines[0]);
        assert!(lines[1].starts_with("summary: device=mic "), "{}", lines[1]);

        assert!(s.flush(3).is_empty(), "window cleared");
        assert_eq!(s.totals()["cam"], 2);
        assert_eq!(s.totals()["mic"], 1);
    }

    #[test]
    fn group_summary_reports_skew_latency_and_per_device_counts() {
        let mut s = GroupSummary::new(Duration::from_secs(10));
        assert!(s.flush().is_none(), "nothing recorded yet");
        s.record(&FrameGroup {
            frames: vec![packet("cam", 0, 10, 8), packet("mic", 4, 10, 8)],
            skew: Duration::from_millis(4),
        });
        s.record(&FrameGroup {
            frames: vec![packet("cam", 33, 40, 8), packet("mic", 45, 50, 8)],
            skew: Duration::from_millis(12),
        });
        let line = s.flush().unwrap();
        assert!(line.starts_with("aligned: "), "{line}");
        assert!(line.contains("groups=2"), "{line}");
        assert!(line.contains("skew_ms mean=8.0 max=12.0"), "{line}");
        assert!(line.contains("latency_ms mean=7.0 max=10.0"), "{line}");
        assert!(line.contains("frames=[cam=2 mic=2]"), "{line}");
        assert!(line.contains("total_groups=2"), "{line}");
        assert_eq!(s.total_groups(), 2);
        assert!(s.flush().is_none(), "window reset");
    }

    #[test]
    fn packet_latency_matches_the_stamps() {
        assert_eq!(
            packet_latency(&packet("cam", 10, 25, 1)),
            Some(Duration::from_millis(15))
        );
    }
}
