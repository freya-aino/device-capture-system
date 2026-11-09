use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::utc::civil;

/// File-name stem for a device id: every character outside `[A-Za-z0-9._-]` becomes `_`, and leading
/// `_` are removed. `/dev/video0` → `dev_video0`.
pub fn file_stem(device_id: &str) -> String {
    let mapped: String = device_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = mapped.trim_start_matches('_');
    if trimmed.is_empty() {
        "device".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Hands out a unique stem per device id within one session. Ids that map to the same stem get `-2`,
/// `-3`, … in the order they first appear.
#[derive(Debug, Default)]
pub struct StemAllocator {
    by_device: HashMap<String, String>,
    used: HashMap<String, u32>,
}

impl StemAllocator {
    pub fn stem(&mut self, device_id: &str) -> String {
        if let Some(stem) = self.by_device.get(device_id) {
            return stem.clone();
        }
        let base = file_stem(device_id);
        let count = self.used.entry(base.clone()).or_insert(0);
        *count += 1;
        let stem = if *count == 1 {
            base
        } else {
            format!("{base}-{count}")
        };
        self.by_device.insert(device_id.to_string(), stem.clone());
        stem
    }
}

/// Session id: the UTC start time as `YYYYMMDDTHHMMSSZ`.
pub fn session_id(start: SystemTime) -> String {
    let (y, mo, d, h, mi, s, _) = civil(start);
    format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{s:02}Z")
}

/// A session directory under `parent` that does not exist yet: `<id>`, else `<id>-2`, `<id>-3`, ….
pub fn session_dir(parent: &Path, start: SystemTime) -> PathBuf {
    let id = session_id(start);
    let mut dir = parent.join(&id);
    let mut n = 2;
    while dir.exists() {
        dir = parent.join(format!("{id}-{n}"));
        n += 1;
    }
    dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn stems_are_safe_file_names() {
        assert_eq!(file_stem("/dev/video0"), "dev_video0");
        assert_eq!(file_stem("sysdefault:CARD=PCH"), "sysdefault_CARD_PCH");
        assert_eq!(file_stem("cam-1.left"), "cam-1.left");
        assert_eq!(file_stem("///"), "device");
    }

    #[test]
    fn colliding_stems_get_a_suffix_and_stay_stable() {
        let mut stems = StemAllocator::default();
        assert_eq!(stems.stem("/dev/video0"), "dev_video0");
        assert_eq!(stems.stem("dev:video0"), "dev_video0-2");
        assert_eq!(stems.stem("/dev/video0"), "dev_video0");
    }

    #[test]
    fn session_id_is_the_utc_start_time() {
        // 2026-11-06T10:15:00Z
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_793_960_100);
        assert_eq!(session_id(t), "20261106T101500Z");
    }

    #[test]
    fn session_dir_skips_existing_directories() {
        let parent = std::env::temp_dir().join(format!("recorder-test-{}", std::process::id()));
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_793_960_100);
        std::fs::create_dir_all(parent.join("20261106T101500Z")).unwrap();
        assert_eq!(session_dir(&parent, t), parent.join("20261106T101500Z-2"));
        std::fs::remove_dir_all(&parent).unwrap();
    }
}
