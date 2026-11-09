use std::net::Ipv4Addr;
use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};

/// Which hub output to forward. Pre-processing lives with the hub, so the sink has no transforms of
/// its own: use `Processed` for a transformed stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveSource {
    Raw,
    Processed,
}

impl LiveSource {
    pub fn default_port(self) -> u16 {
        match self {
            LiveSource::Raw => 5556,
            LiveSource::Processed => 5558,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveConfig {
    pub hub_ip: Ipv4Addr,
    pub source: LiveSource,
    pub hub_port: u16,
    pub device: String,
    pub sink: PathBuf,
}

impl LiveConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Reads the `LIVE_*` variables through `lookup`. `LIVE_DEVICE` and `LIVE_SINK` are required; set
    /// but invalid values are errors.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let source = match lookup("LIVE_SOURCE").as_deref() {
            None | Some("raw") => LiveSource::Raw,
            Some("processed") => LiveSource::Processed,
            Some(other) => bail!("LIVE_SOURCE must be raw or processed, got {other:?}"),
        };
        let device = lookup("LIVE_DEVICE")
            .filter(|d| !d.trim().is_empty())
            .ok_or_else(|| anyhow!("LIVE_DEVICE is required: the device id to forward"))?;
        let sink = lookup("LIVE_SINK")
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                anyhow!("LIVE_SINK is required: the v4l2loopback device, e.g. /dev/video10")
            })?;
        let hub_ip = match lookup("LIVE_HUB_IP") {
            Some(raw) => raw
                .parse()
                .map_err(|e| anyhow!("invalid LIVE_HUB_IP {raw:?}: {e}"))?,
            None => Ipv4Addr::LOCALHOST,
        };
        let hub_port = match lookup("LIVE_HUB_PORT") {
            Some(raw) => raw
                .parse()
                .map_err(|e| anyhow!("invalid LIVE_HUB_PORT {raw:?}: {e}"))?,
            None => source.default_port(),
        };
        Ok(LiveConfig {
            hub_ip,
            source,
            hub_port,
            device: device.trim().to_string(),
            sink: PathBuf::from(sink),
        })
    }

    pub fn hub_endpoint(&self) -> String {
        format!("tcp://{}:{}", self.hub_ip, self.hub_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(vars: &[(&str, &str)]) -> Result<LiveConfig> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        LiveConfig::from_lookup(|key| vars.get(key).cloned())
    }

    #[test]
    fn defaults_read_the_raw_stream_on_this_host() {
        let c = config(&[
            ("LIVE_DEVICE", "/dev/video0"),
            ("LIVE_SINK", "/dev/video10"),
        ])
        .unwrap();
        assert_eq!(c.source, LiveSource::Raw);
        assert_eq!(c.hub_endpoint(), "tcp://127.0.0.1:5556");
        assert_eq!(
            (c.device.as_str(), c.sink),
            ("/dev/video0", PathBuf::from("/dev/video10"))
        );
    }

    #[test]
    fn processed_source_uses_the_preprocess_port() {
        let c = config(&[
            ("LIVE_DEVICE", "cam"),
            ("LIVE_SINK", "/dev/video10"),
            ("LIVE_SOURCE", "processed"),
        ])
        .unwrap();
        assert_eq!(c.hub_port, 5558);
    }

    #[test]
    fn missing_or_invalid_values_are_errors() {
        for vars in [
            vec![("LIVE_SINK", "/dev/video10")],
            vec![("LIVE_DEVICE", "cam")],
            vec![
                ("LIVE_DEVICE", "cam"),
                ("LIVE_SINK", "/dev/video10"),
                ("LIVE_SOURCE", "aligned"),
            ],
            vec![
                ("LIVE_DEVICE", "cam"),
                ("LIVE_SINK", "/dev/video10"),
                ("LIVE_HUB_PORT", "x"),
            ],
        ] {
            assert!(config(&vars).is_err(), "accepted {vars:?}");
        }
    }
}
