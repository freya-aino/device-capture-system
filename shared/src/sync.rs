use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, SystemTime};

/// One frame observed by the hub, identified by its producer device and capture time.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub device_id: String,
    pub time: SystemTime,
}

/// A set of samples, at most one per device, whose capture times all fall within the tolerance.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignedGroup {
    pub samples: Vec<Sample>,
    pub skew: Duration,
}

/// Groups samples from several devices into aligned sets by capture time.
///
/// A group is emitted once every expected device has a pending sample within `tolerance` of the
/// earliest pending sample. Samples that trail the newest arrival by more than `tolerance` can no
/// longer pair with anything, so they are dropped and counted; a silent device therefore cannot
/// stall the others.
#[derive(Debug)]
pub struct Aligner {
    devices: Vec<String>,
    tolerance: Duration,
    pending: BTreeMap<String, VecDeque<SystemTime>>,
    dropped: u64,
}

impl Aligner {
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
    pub fn push(&mut self, sample: Sample) -> Option<AlignedGroup> {
        let queue = self.pending.get_mut(&sample.device_id)?;
        queue.push_back(sample.time);
        self.drop_stale(sample.time);
        self.try_emit()
    }

    fn try_emit(&mut self) -> Option<AlignedGroup> {
        let anchor = self
            .pending
            .values()
            .filter_map(|queue| queue.front().copied())
            .min()?;

        let mut picks = Vec::with_capacity(self.devices.len());
        for device in &self.devices {
            let queue = self.pending.get(device)?;
            let best = queue
                .iter()
                .enumerate()
                .filter(|(_, time)| within(**time, anchor, self.tolerance))
                .min_by_key(|(_, time)| abs_diff(**time, anchor));
            match best {
                Some((index, time)) => picks.push((device.clone(), index, *time)),
                None => return None,
            }
        }

        let mut samples = Vec::with_capacity(picks.len());
        let mut earliest = anchor;
        let mut latest = anchor;
        for (device, index, time) in picks {
            self.pending.get_mut(&device)?.drain(..=index);
            earliest = earliest.min(time);
            latest = latest.max(time);
            samples.push(Sample {
                device_id: device,
                time,
            });
        }

        let skew = latest.duration_since(earliest).unwrap_or_default();
        Some(AlignedGroup { samples, skew })
    }

    /// Removes samples that trail `newest` by more than the tolerance and counts them as dropped.
    fn drop_stale(&mut self, newest: SystemTime) {
        for queue in self.pending.values_mut() {
            while let Some(front) = queue.front() {
                if *front + self.tolerance < newest {
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

    fn sample(device: &str, ms: u64) -> Sample {
        Sample {
            device_id: device.to_string(),
            time: at(ms),
        }
    }

    fn aligner() -> Aligner {
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
}
