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
/// earliest pending sample. Samples that trail the newest arrival by more than `tolerance` can no
/// longer pair with anything, so they are dropped and counted; a silent device therefore cannot
/// stall the others.
#[derive(Debug)]
pub struct Aligner<T> {
    devices: Vec<String>,
    tolerance: Duration,
    pending: BTreeMap<String, VecDeque<(SystemTime, T)>>,
    dropped: u64,
}

impl<T> Aligner<T> {
    pub fn new(devices: Vec<String>, tolerance: Duration) -> Self {
        let pending = devices
            .iter()
            .map(|id| (id.clone(), VecDeque::new()))
            .collect();
        Aligner {
            devices,
            tolerance,
            pending,
            dropped: 0,
        }
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
        let anchor = self
            .pending
            .values()
            .filter_map(|queue| queue.front().map(|entry| entry.0))
            .min()?;

        let mut picks = Vec::with_capacity(self.devices.len());
        for device in &self.devices {
            let queue = self.pending.get(device)?;
            let best = queue
                .iter()
                .enumerate()
                .filter(|(_, entry)| within(entry.0, anchor, self.tolerance))
                .min_by_key(|(_, entry)| abs_diff(entry.0, anchor));
            match best {
                Some((index, entry)) => picks.push((device.clone(), index, entry.0)),
                None => return None,
            }
        }

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
        Some(AlignedGroup { samples, skew })
    }

    /// Removes samples that trail `newest` by more than the tolerance and counts them as dropped.
    fn drop_stale(&mut self, newest: SystemTime) {
        for queue in self.pending.values_mut() {
            while let Some((time, _)) = queue.front() {
                if *time + self.tolerance < newest {
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
    fn returns_the_payload_of_each_grouped_sample() {
        let mut aligner: Aligner<&str> =
            Aligner::new(vec!["cam".into(), "mic".into()], Duration::from_millis(20));
        assert!(aligner
            .push(Sample {
                device_id: "cam".into(),
                time: at(1000),
                payload: "cam-frame",
            })
            .is_none());
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
