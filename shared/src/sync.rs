use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, SystemTime};

/// One frame observed by the hub, identified by its producer device and capture time.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample<T> {
    pub device_id: String,
    pub time: SystemTime,
    pub payload: T,
}

/// A set of samples, at most one per device, whose capture times all fall within the tolerance.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignedGroup<T> {
    pub samples: Vec<Sample<T>>,
    pub skew: Duration,
}

/// Groups samples from several devices into aligned sets by capture time.
///
/// A group is emitted once every expected device has a pending sample within `tolerance` of the
/// earliest pending sample (the anchor). Two rules keep the queues from stalling:
///
/// - When another device has already delivered samples that are all later than
///   `anchor + tolerance`, the anchor can never complete (per-device streams are time-ordered), so
///   it is dropped and counted at once. This is independent of how late frames arrive over the
///   network, which matters: on real hardware a 1080p camera frame reaches the hub ~20 ms after a
///   microphone packet with the same capture time.
/// - Samples that trail the newest arrival by more than `max_wait` are dropped, so a silent device
///   cannot hold the others' frames for ever. `max_wait` (default `DEFAULT_MAX_WAIT`) must cover the
///   transport-latency difference between devices; it is not the pairing tolerance.
#[derive(Debug)]
pub struct Aligner<T> {
    devices: Vec<String>,
    tolerance: Duration,
    max_wait: Duration,
    pending: BTreeMap<String, VecDeque<(SystemTime, T)>>,
    dropped: u64,
}

/// Default for how long a sample may wait for partners before it is given up on.
pub const DEFAULT_MAX_WAIT: Duration = Duration::from_millis(200);

impl<T> Aligner<T> {
    pub fn new(devices: Vec<String>, tolerance: Duration) -> Self {
        let pending = devices
            .iter()
            .map(|id| (id.clone(), VecDeque::new()))
            .collect();
        Aligner {
            devices,
            tolerance,
            max_wait: DEFAULT_MAX_WAIT.max(tolerance),
            pending,
            dropped: 0,
        }
    }

    /// Sets how long a sample may wait for partners. Values below `tolerance` are raised to it.
    pub fn with_max_wait(mut self, max_wait: Duration) -> Self {
        self.max_wait = max_wait.max(self.tolerance);
        self
    }

    pub fn max_wait(&self) -> Duration {
        self.max_wait
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Adds one sample and returns the group it completes, if any. Samples from devices that were
    /// not registered are ignored.
    pub fn push(&mut self, sample: Sample<T>) -> Option<AlignedGroup<T>> {
        let Sample {
            device_id,
            time,
            payload,
        } = sample;
        let queue = self.pending.get_mut(&device_id)?;
        queue.push_back((time, payload));
        self.drop_stale(time);
        self.try_emit()
    }

    fn try_emit(&mut self) -> Option<AlignedGroup<T>> {
        loop {
            let anchor = self
                .pending
                .values()
                .filter_map(|queue| queue.front().map(|entry| entry.0))
                .min()?;

            let mut picks = Vec::with_capacity(self.devices.len());
            let mut hopeless = false;
            for device in &self.devices {
                let queue = self.pending.get(device)?;
                let best = queue
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| within(entry.0, anchor, self.tolerance))
                    .min_by_key(|(_, entry)| abs_diff(entry.0, anchor));
                match best {
                    Some((index, entry)) => picks.push((device.clone(), index, entry.0)),
                    // This device has samples, all of them later than the anchor's window, and
                    // its later samples are later still: the anchor will never complete.
                    None if !queue.is_empty() => hopeless = true,
                    // This device has not delivered yet; keep waiting (bounded by `max_wait`).
                    None => {}
                }
            }

            if picks.len() == self.devices.len() {
                let mut samples = Vec::with_capacity(picks.len());
                let mut earliest = anchor;
                let mut latest = anchor;
                for (device, index, time) in picks {
                    let (_, payload) = self.pending.get_mut(&device)?.drain(..=index).last()?;
                    earliest = earliest.min(time);
                    latest = latest.max(time);
                    samples.push(Sample {
                        device_id: device,
                        time,
                        payload,
                    });
                }
                let skew = latest.duration_since(earliest).unwrap_or_default();
                return Some(AlignedGroup { samples, skew });
            }

            if !hopeless {
                return None;
            }
            // Drop the anchor sample (the earliest pending one) and try the next candidate.
            for queue in self.pending.values_mut() {
                if queue.front().is_some_and(|entry| entry.0 == anchor) {
                    queue.pop_front();
                    self.dropped += 1;
                    break;
                }
            }
        }
    }

    /// Removes samples that trail `newest` by more than `max_wait` and counts them as dropped.
    fn drop_stale(&mut self, newest: SystemTime) {
        for queue in self.pending.values_mut() {
            while let Some((time, _)) = queue.front() {
                if *time + self.max_wait < newest {
                    queue.pop_front();
                    self.dropped += 1;
                } else {
                    break;
                }
            }
        }
    }
}

fn within(time: SystemTime, anchor: SystemTime, tolerance: Duration) -> bool {
    abs_diff(time, anchor) <= tolerance
}

fn abs_diff(a: SystemTime, b: SystemTime) -> Duration {
    match a.duration_since(b) {
        Ok(d) => d,
        Err(e) => e.duration(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_millis(ms)
    }

    fn sample(device: &str, ms: u64) -> Sample<()> {
        Sample {
            device_id: device.to_string(),
            time: at(ms),
            payload: (),
        }
    }

    fn aligner() -> Aligner<()> {
        Aligner::new(vec!["cam".into(), "mic".into()], Duration::from_millis(20))
    }

    #[test]
    fn emits_nothing_until_every_device_has_a_sample() {
        let mut aligner = aligner();
        assert!(aligner.push(sample("cam", 1000)).is_none());
    }

    #[test]
    fn groups_samples_within_tolerance_and_reports_skew() {
        let mut aligner = aligner();
        assert!(aligner.push(sample("cam", 1000)).is_none());
        let group = aligner
            .push(sample("mic", 1010))
            .expect("group should be emitted");

        assert_eq!(group.samples.len(), 2);
        assert_eq!(group.skew, Duration::from_millis(10));
        assert_eq!(aligner.dropped(), 0);
    }

    #[test]
    fn does_not_group_samples_outside_tolerance() {
        let mut aligner = aligner();
        assert!(aligner.push(sample("cam", 1000)).is_none());
        assert!(aligner.push(sample("mic", 1100)).is_none());
    }

    #[test]
    fn ignores_samples_from_unknown_devices() {
        let mut aligner = aligner();
        assert!(aligner.push(sample("other", 1000)).is_none());
        assert!(aligner.push(sample("cam", 1000)).is_none());
        assert!(aligner.push(sample("mic", 1000)).is_some());
    }

    #[test]
    fn pairs_each_device_sample_at_most_once() {
        let mut aligner = aligner();
        assert!(aligner.push(sample("cam", 1000)).is_none());
        assert!(aligner.push(sample("cam", 1005)).is_none());
        let group = aligner
            .push(sample("mic", 1002))
            .expect("group should be emitted");
        assert_eq!(group.samples.len(), 2);

        let next = aligner
            .push(sample("mic", 1006))
            .expect("second group should be emitted");
        assert_eq!(
            next.samples.iter().filter(|s| s.device_id == "cam").count(),
            1
        );
    }

    #[test]
    fn drops_a_stale_sample_that_never_found_a_partner() {
        let mut aligner = aligner();
        assert!(aligner.push(sample("cam", 1000)).is_none());
        assert!(aligner.push(sample("mic", 2000)).is_none());
        assert_eq!(aligner.dropped(), 1);
    }

    #[test]
    fn drops_an_anchor_once_every_other_device_has_moved_past_it() {
        let mut aligner = aligner();
        assert!(aligner.push(sample("cam", 1000)).is_none());
        // 1030 is outside the 20 ms window of 1000 and the mic stream is time-ordered, so the
        // camera frame at 1000 can never complete. It is dropped at once, well inside max_wait.
        assert!(aligner.push(sample("mic", 1030)).is_none());
        assert_eq!(aligner.dropped(), 1);
        // The next camera frame pairs with the waiting microphone packet.
        let group = aligner.push(sample("cam", 1035)).expect("group");
        assert_eq!(group.skew, Duration::from_millis(5));
        assert_eq!(aligner.dropped(), 1);
    }

    #[test]
    fn silent_device_releases_others_after_max_wait() {
        let mut aligner = aligner().with_max_wait(Duration::from_millis(100));
        assert_eq!(aligner.max_wait(), Duration::from_millis(100));
        assert!(aligner.push(sample("cam", 1000)).is_none());
        assert!(aligner.push(sample("cam", 1033)).is_none());
        assert_eq!(
            aligner.dropped(),
            0,
            "within max_wait nothing is dropped yet"
        );
        assert!(aligner.push(sample("cam", 1130)).is_none());
        assert_eq!(
            aligner.dropped(),
            1,
            "1000 trails 1130 by more than 100 ms, 1033 does not"
        );
    }

    #[test]
    fn max_wait_is_never_below_tolerance() {
        let aligner = aligner().with_max_wait(Duration::from_millis(1));
        assert_eq!(aligner.max_wait(), Duration::from_millis(20));
        assert_eq!(
            super::super::sync::DEFAULT_MAX_WAIT,
            Duration::from_millis(200)
        );
    }

    /// Replays the timing measured on real hardware (docs/hardware/2026-10-30-framework-12-webcam.md):
    /// camera frames every 33.3 ms arriving ~25 ms after capture, microphone packets every 32 ms
    /// arriving ~5 ms after capture. Pushed in arrival order, nearly every camera frame must pair.
    #[test]
    fn transport_lag_between_devices_does_not_lose_groups() {
        let mut arrivals: Vec<(u64, &str, u64)> = Vec::new(); // (arrival, device, capture)
        for n in 0..300u64 {
            let capture = n * 1000 / 30;
            arrivals.push((capture + 25, "cam", capture));
        }
        for n in 0..313u64 {
            let capture = n * 32;
            arrivals.push((capture + 5, "mic", capture));
        }
        arrivals.sort();

        let mut aligner = aligner();
        let mut groups = 0;
        for (_, device, capture) in arrivals {
            if aligner.push(sample(device, 1_000_000 + capture)).is_some() {
                groups += 1;
            }
        }
        assert!(
            groups >= 290,
            "only {groups} groups out of 300 camera frames"
        );
    }

    #[test]
    fn returns_the_payload_of_each_grouped_sample() {
        let mut aligner: Aligner<&str> =
            Aligner::new(vec!["cam".into(), "mic".into()], Duration::from_millis(20));
        assert!(
            aligner
                .push(Sample {
                    device_id: "cam".into(),
                    time: at(1000),
                    payload: "cam-frame",
                })
                .is_none()
        );
        let group = aligner
            .push(Sample {
                device_id: "mic".into(),
                time: at(1005),
                payload: "mic-frame",
            })
            .expect("group should be emitted");

        let payloads: Vec<&str> = group.samples.iter().map(|s| s.payload).collect();
        assert!(payloads.contains(&"cam-frame"));
        assert!(payloads.contains(&"mic-frame"));
    }
}
