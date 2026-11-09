//! A saved node configuration (`docs/step-27-saved-config.md`): the device by id and the resolved
//! mode, so a capture host starts the same way every time. Environment variables never override a
//! value in the file.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use shared::{CameraConfig, Device, MicrophoneConfig};

/// Frame rates closer than this count as the same advertised mode (`30000/1001` vs `29.97`).
pub const FPS_TOLERANCE: f64 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeDeviceType {
    Camera,
    Microphone,
}

/// The resolved camera mode, not a selection policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraSection {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub pixel_format: String,
}

impl CameraSection {
    pub fn from_config(config: &CameraConfig) -> Self {
        CameraSection {
            width: config.width,
            height: config.height,
            fps: config.frames_per_second(),
            pixel_format: config.fourcc_string(),
        }
    }

    /// The advertised config this section names, or an error when the camera no longer offers it.
    pub fn matching(&self, advertised: &[CameraConfig]) -> Result<CameraConfig> {
        advertised
            .iter()
            .find(|c| {
                c.width == self.width
                    && c.height == self.height
                    && c.fourcc_string() == self.pixel_format
                    && (c.frames_per_second() - self.fps).abs() < FPS_TOLERANCE
            })
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "the camera no longer advertises {}x{} @ {} fps {} ({} modes advertised)",
                    self.width,
                    self.height,
                    self.fps,
                    self.pixel_format,
                    advertised.len()
                )
            })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicrophoneSection {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: u32,
}

impl MicrophoneSection {
    pub fn from_config(config: &MicrophoneConfig) -> Self {
        MicrophoneSection {
            sample_rate: config.sample_rate,
            channels: config.channels,
            buffer_size: config.buffer_size,
        }
    }

    pub fn to_config(&self) -> MicrophoneConfig {
        MicrophoneConfig {
            sample_rate: self.sample_rate,
            channels: self.channels,
            buffer_size: self.buffer_size,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeConfig {
    pub device_type: NodeDeviceType,
    pub device_id: String,
    /// The address the node publishes on, `ip:port`.
    pub bind: String,
    pub status_addr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<CameraSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub microphone: Option<MicrophoneSection>,
}

impl NodeConfig {
    pub fn camera(
        device_id: &str,
        config: &CameraConfig,
        bind: SocketAddrV4,
        status_addr: &str,
    ) -> Self {
        NodeConfig {
            device_type: NodeDeviceType::Camera,
            device_id: device_id.to_string(),
            bind: bind.to_string(),
            status_addr: status_addr.to_string(),
            camera: Some(CameraSection::from_config(config)),
            microphone: None,
        }
    }

    pub fn microphone(
        device_id: &str,
        config: &MicrophoneConfig,
        bind: SocketAddrV4,
        status_addr: &str,
    ) -> Self {
        NodeConfig {
            device_type: NodeDeviceType::Microphone,
            device_id: device_id.to_string(),
            bind: bind.to_string(),
            status_addr: status_addr.to_string(),
            camera: None,
            microphone: Some(MicrophoneSection::from_config(config)),
        }
    }

    /// Parses and validates: unknown keys, a missing section for the device type, a section for
    /// the other type, and an unparsable `bind` are errors.
    pub fn from_toml(text: &str) -> Result<Self> {
        let config: NodeConfig = toml::from_str(text).context("parsing the node config")?;
        config.validate()?;
        Ok(config)
    }

    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("node config serializes")
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::from_toml(&text).with_context(|| format!("in {}", path.display()))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        self.validate()?;
        std::fs::write(path, self.to_toml()).with_context(|| format!("writing {}", path.display()))
    }

    fn validate(&self) -> Result<()> {
        match self.device_type {
            NodeDeviceType::Camera => {
                ensure!(
                    self.camera.is_some(),
                    "device_type is camera but [camera] is missing"
                );
                ensure!(
                    self.microphone.is_none(),
                    "device_type is camera but a [microphone] section is present"
                );
            }
            NodeDeviceType::Microphone => {
                ensure!(
                    self.microphone.is_some(),
                    "device_type is microphone but [microphone] is missing"
                );
                ensure!(
                    self.camera.is_none(),
                    "device_type is microphone but a [camera] section is present"
                );
            }
        }
        ensure!(!self.device_id.trim().is_empty(), "device_id is empty");
        self.bind_addr()?;
        Ok(())
    }

    pub fn bind_addr(&self) -> Result<SocketAddrV4> {
        self.bind
            .parse()
            .map_err(|e| anyhow!("bind must be ip:port, got {:?}: {e}", self.bind))
    }

    pub fn bind_ip(&self) -> Result<Ipv4Addr> {
        Ok(*self.bind_addr()?.ip())
    }

    pub fn bind_port(&self) -> Result<u16> {
        Ok(self.bind_addr()?.port())
    }

    pub fn camera_section(&self) -> Result<&CameraSection> {
        self.camera
            .as_ref()
            .ok_or_else(|| anyhow!("config has no [camera] section"))
    }

    pub fn microphone_section(&self) -> Result<&MicrophoneSection> {
        self.microphone
            .as_ref()
            .ok_or_else(|| anyhow!("config has no [microphone] section"))
    }
}

/// The enumerated device with the saved id, by value so it can be started. Never falls back to
/// another device.
pub fn take_device_by_id<D: Device>(devices: Vec<D>, device_id: &str) -> Result<D> {
    let available: Vec<String> = devices.iter().map(|d| d.id().to_string()).collect();
    devices
        .into_iter()
        .find(|d| d.id() == device_id)
        .ok_or_else(|| {
            anyhow!(
                "device {device_id:?} is not present; available: {}",
                if available.is_empty() {
                    "none".to_string()
                } else {
                    available.join(", ")
                }
            )
        })
}

/// The exact camera mode a saved config names among the modes the camera advertises now.
pub fn resolve_camera_mode<D: Device<Config = CameraConfig>>(
    camera: &D,
    section: &CameraSection,
) -> Result<CameraConfig> {
    let advertised = camera.get_configs()?;
    if advertised.is_empty() {
        bail!("camera {:?} advertises no capture configs", camera.id());
    }
    section
        .matching(&advertised)
        .with_context(|| format!("camera {:?}", camera.id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{DeviceStatus, DeviceType, FramePacket};
    use std::thread::JoinHandle;

    fn cam(width: u32, height: u32, fps: (u32, u32), fourcc: &str) -> CameraConfig {
        let chars: Vec<char> = fourcc.chars().collect();
        CameraConfig {
            width,
            height,
            fps,
            fourcc: [chars[0], chars[1], chars[2], chars[3]],
        }
    }

    struct FakeCamera {
        id: String,
        modes: Vec<CameraConfig>,
        device_type: DeviceType,
        status: DeviceStatus,
    }

    impl FakeCamera {
        fn new(id: &str, modes: Vec<CameraConfig>) -> Self {
            FakeCamera {
                id: id.to_string(),
                modes,
                device_type: DeviceType::Camera,
                status: DeviceStatus::Available,
            }
        }
    }

    impl Device for FakeCamera {
        type Config = CameraConfig;
        fn id(&self) -> &str {
            &self.id
        }
        fn name(&self) -> &str {
            "fake"
        }
        fn device_type(&self) -> &DeviceType {
            &self.device_type
        }
        fn device_status(&self) -> &DeviceStatus {
            &self.status
        }
        fn get_configs(&self) -> Result<Vec<CameraConfig>> {
            Ok(self.modes.clone())
        }
        fn start(
            &mut self,
            _: CameraConfig,
            _: Box<dyn Fn(FramePacket) -> Result<()> + Send + 'static>,
        ) -> Result<JoinHandle<Result<()>>> {
            unimplemented!("fake")
        }
        fn stop(&mut self) -> Result<()> {
            Ok(())
        }
    }

    const BIND: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5555);

    #[test]
    fn camera_config_round_trips_through_toml() {
        let config = NodeConfig::camera(
            "/dev/video0",
            &cam(1920, 1080, (1, 30), "MJPG"),
            BIND,
            "127.0.0.1:8080",
        );
        let text = config.to_toml();
        assert!(text.contains("device_type = \"camera\""), "{text}");
        assert!(text.contains("[camera]"), "{text}");
        assert!(!text.contains("microphone"), "{text}");
        let back = NodeConfig::from_toml(&text).unwrap();
        assert_eq!(back, config);
        assert_eq!(back.bind_ip().unwrap(), Ipv4Addr::UNSPECIFIED);
        assert_eq!(back.bind_port().unwrap(), 5555);
        assert_eq!(back.camera_section().unwrap().pixel_format, "MJPG");
    }

    #[test]
    fn microphone_config_round_trips_and_the_spec_example_parses() {
        let config = NodeConfig::microphone(
            "sysdefault:CARD=PCH",
            &MicrophoneConfig {
                sample_rate: 16000,
                channels: 1,
                buffer_size: 512,
            },
            BIND,
            "127.0.0.1:8080",
        );
        let back = NodeConfig::from_toml(&config.to_toml()).unwrap();
        assert_eq!(back, config);
        assert_eq!(
            back.microphone_section().unwrap().to_config().buffer_size,
            512
        );

        let spec = r#"
device_type = "camera"
device_id = "/dev/video0"
bind = "0.0.0.0:5555"
status_addr = "127.0.0.1:8080"

[camera]
width = 1920
height = 1080
fps = 30
pixel_format = "MJPG"
"#;
        let parsed = NodeConfig::from_toml(spec).unwrap();
        assert_eq!(
            parsed.camera_section().unwrap().fps,
            30.0,
            "integer fps is accepted"
        );
    }

    #[test]
    fn unknown_keys_and_mismatched_sections_are_rejected() {
        let base = NodeConfig::camera("/dev/video0", &cam(640, 480, (1, 30), "YUYV"), BIND, "a:1");
        let mut unknown = base.to_toml();
        unknown.push_str("\ncolour = \"red\"\n");
        assert!(NodeConfig::from_toml(&unknown).is_err(), "unknown key");

        let missing = base.to_toml().replace("[camera]", "[camera_old]");
        assert!(NodeConfig::from_toml(&missing).is_err(), "missing section");

        let both = format!(
            "{}\n[microphone]\nsample_rate = 16000\nchannels = 1\nbuffer_size = 512\n",
            base.to_toml()
        );
        assert!(
            NodeConfig::from_toml(&both).is_err(),
            "section for the other type"
        );

        let bad_bind = base.to_toml().replace("0.0.0.0:5555", "everywhere");
        assert!(
            NodeConfig::from_toml(&bad_bind).is_err(),
            "bind must be ip:port"
        );
    }

    #[test]
    fn devices_are_found_by_id_and_never_substituted() {
        let devices = vec![
            FakeCamera::new("/dev/video0", vec![]),
            FakeCamera::new("/dev/video2", vec![]),
        ];
        assert_eq!(
            take_device_by_id(devices, "/dev/video2").unwrap().id,
            "/dev/video2"
        );
        let devices = vec![FakeCamera::new("/dev/video0", vec![])];
        let err = take_device_by_id(devices, "/dev/video2").err().unwrap();
        assert!(err.to_string().contains("/dev/video0"), "{err}");
        let err = take_device_by_id(Vec::<FakeCamera>::new(), "x")
            .err()
            .unwrap();
        assert!(err.to_string().contains("none"), "{err}");
    }

    #[test]
    fn the_saved_mode_must_still_be_advertised() {
        let advertised = vec![
            cam(1920, 1080, (1001, 30000), "MJPG"),
            cam(640, 480, (1, 30), "YUYV"),
        ];
        let camera = FakeCamera::new("/dev/video0", advertised.clone());

        let section = CameraSection {
            width: 1920,
            height: 1080,
            fps: 29.97,
            pixel_format: "MJPG".into(),
        };
        let mode = resolve_camera_mode(&camera, &section).unwrap();
        assert_eq!((mode.width, mode.fps), (1920, (1001, 30000)));

        for (w, h, fps, f) in [
            (1920, 1080, 30.0, "YUYV"),
            (1280, 720, 30.0, "MJPG"),
            (640, 480, 15.0, "YUYV"),
        ] {
            let section = CameraSection {
                width: w,
                height: h,
                fps,
                pixel_format: f.into(),
            };
            assert!(
                resolve_camera_mode(&camera, &section).is_err(),
                "{w}x{h}@{fps} {f} is not advertised"
            );
        }
        let empty = FakeCamera::new("/dev/video9", vec![]);
        assert!(resolve_camera_mode(&empty, &section).is_err());
    }
}
