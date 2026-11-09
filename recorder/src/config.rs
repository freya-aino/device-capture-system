use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};

/// Which hub output to record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The raw stream (`HUB_CONSUMER_PORT`).
    Raw,
    /// Aligned groups (`HUB_SYNC_PORT`).
    Aligned,
    /// Pre-processed frames (`HUB_PREPROCESS_PORT`).
    Processed,
}

impl Source {
    pub fn default_port(self) -> u16 {
        match self {
            Source::Raw => 5556,
            Source::Aligned => 5557,
            Source::Processed => 5558,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Source::Raw => "raw",
            Source::Aligned => "aligned",
            Source::Processed => "processed",
        }
    }
}

/// Encoder for uncompressed camera input. MJPG input is always stream-copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Ffv1,
    H264,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecorderConfig {
    pub hub_ip: std::net::Ipv4Addr,
    pub source: Source,
    pub hub_port: u16,
    pub devices: Vec<String>,
    pub dir: PathBuf,
    /// `None` records until stopped.
    pub duration: Option<Duration>,
    pub codec: Codec,
    pub calibration: Option<PathBuf>,
}

impl RecorderConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Reads the `RECORD_*` variables through `lookup`. Unset variables take their defaults; set but
    /// invalid ones are errors.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let source = match lookup("RECORD_SOURCE").as_deref() {
            None | Some("raw") => Source::Raw,
            Some("aligned") => Source::Aligned,
            Some("processed") => Source::Processed,
            Some(other) => bail!("RECORD_SOURCE must be raw, aligned or processed, got {other:?}"),
        };
        let codec = match lookup("RECORD_CODEC").as_deref() {
            None | Some("ffv1") => Codec::Ffv1,
            Some("h264") => Codec::H264,
            Some(other) => bail!("RECORD_CODEC must be ffv1 or h264, got {other:?}"),
        };
        let devices: Vec<String> = lookup("RECORD_DEVICES")
            .ok_or_else(|| anyhow!("RECORD_DEVICES is required: the device ids to record"))?
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .collect();
        if devices.is_empty() {
            bail!("RECORD_DEVICES contains no device ids");
        }
        let secs: u64 = parse(&lookup, "RECORD_SECS")?.unwrap_or(0);
        Ok(RecorderConfig {
            hub_ip: parse(&lookup, "RECORD_HUB_IP")?.unwrap_or(std::net::Ipv4Addr::LOCALHOST),
            source,
            hub_port: parse(&lookup, "RECORD_HUB_PORT")?.unwrap_or(source.default_port()),
            devices,
            dir: lookup("RECORD_DIR").map_or_else(|| PathBuf::from("recordings"), PathBuf::from),
            duration: (secs > 0).then(|| Duration::from_secs(secs)),
            codec,
            calibration: lookup("RECORD_CALIBRATION").map(PathBuf::from),
        })
    }

    pub fn hub_endpoint(&self) -> String {
        format!("tcp://{}:{}", self.hub_ip, self.hub_port)
    }
}

fn parse<T: std::str::FromStr>(
    lookup: &impl Fn(&str) -> Option<String>,
    key: &str,
) -> Result<Option<T>>
where
    T::Err: std::fmt::Display,
{
    lookup(key)
        .map(|raw| {
            raw.parse::<T>()
                .map_err(|e| anyhow!("invalid {key} {raw:?}: {e}"))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(vars: &[(&str, &str)]) -> Result<RecorderConfig> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        RecorderConfig::from_lookup(|key| vars.get(key).cloned())
    }

    #[test]
    fn defaults_need_only_the_devices() {
        let c = config(&[("RECORD_DEVICES", "/dev/video0, sysdefault:CARD=PCH ,")]).unwrap();
        assert_eq!(c.devices, vec!["/dev/video0", "sysdefault:CARD=PCH"]);
        assert_eq!(c.source, Source::Raw);
        assert_eq!(c.hub_endpoint(), "tcp://127.0.0.1:5556");
        assert_eq!(c.dir, PathBuf::from("recordings"));
        assert_eq!(
            (c.duration, c.codec, c.calibration),
            (None, Codec::Ffv1, None)
        );
    }

    #[test]
    fn the_source_picks_the_default_port_and_the_port_overrides_it() {
        let c = config(&[("RECORD_DEVICES", "a"), ("RECORD_SOURCE", "aligned")]).unwrap();
        assert_eq!(c.hub_port, 5557);
        let c = config(&[
            ("RECORD_DEVICES", "a"),
            ("RECORD_SOURCE", "processed"),
            ("RECORD_HUB_PORT", "6000"),
            ("RECORD_HUB_IP", "10.0.0.5"),
        ])
        .unwrap();
        assert_eq!(c.hub_endpoint(), "tcp://10.0.0.5:6000");
    }

    #[test]
    fn optional_values_are_read() {
        let c = config(&[
            ("RECORD_DEVICES", "a"),
            ("RECORD_SECS", "30"),
            ("RECORD_CODEC", "h264"),
            ("RECORD_DIR", "/data/sessions"),
            ("RECORD_CALIBRATION", "cal.json"),
        ])
        .unwrap();
        assert_eq!(c.duration, Some(Duration::from_secs(30)));
        assert_eq!(c.codec, Codec::H264);
        assert_eq!(c.dir, PathBuf::from("/data/sessions"));
        assert_eq!(c.calibration, Some(PathBuf::from("cal.json")));
    }

    #[test]
    fn invalid_values_are_errors_not_defaults() {
        for vars in [
            vec![],
            vec![("RECORD_DEVICES", " , ")],
            vec![("RECORD_DEVICES", "a"), ("RECORD_SOURCE", "live")],
            vec![("RECORD_DEVICES", "a"), ("RECORD_CODEC", "vp9")],
            vec![("RECORD_DEVICES", "a"), ("RECORD_SECS", "ten")],
            vec![("RECORD_DEVICES", "a"), ("RECORD_HUB_IP", "hub.local")],
        ] {
            assert!(config(&vars).is_err(), "accepted {vars:?}");
        }
    }
}
