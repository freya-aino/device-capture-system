//! The TOML configuration (one pipeline per device id) and the `Preprocessor` that applies it to
//! frame packets.
//!
//! ```toml
//! [[device]]
//! id = "camera-0"
//! steps = [
//!   { rotate = 90 },
//!   { flip = "horizontal" },
//!   { crop = { x = 0, y = 0, width = 640, height = 480 } },
//!   { resize = { width = 320, height = 240 } },
//!   { brightness = 10 },
//!   { contrast = 1.2 },
//!   { gamma = 0.8 },
//!   "grayscale",
//! ]
//! ```

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use shared::{DeviceType, FramePacket};

use crate::decode::decode_payload;
use crate::transform::{FlipAxis, Pipeline, Rotation, Transform};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    device: Vec<RawDevice>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDevice {
    id: String,
    steps: Vec<RawStep>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
enum RawStep {
    Rotate(u32),
    Flip(String),
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Resize {
        width: u32,
        height: u32,
    },
    Brightness(i32),
    Contrast(f32),
    Gamma(f32),
    Grayscale,
}

impl RawStep {
    fn validate(self) -> Result<Transform> {
        Ok(match self {
            RawStep::Rotate(90) => Transform::Rotate(Rotation::Deg90),
            RawStep::Rotate(180) => Transform::Rotate(Rotation::Deg180),
            RawStep::Rotate(270) => Transform::Rotate(Rotation::Deg270),
            RawStep::Rotate(other) => bail!("rotate must be 90, 180 or 270, got {other}"),
            RawStep::Flip(axis) => match axis.as_str() {
                "horizontal" => Transform::Flip(FlipAxis::Horizontal),
                "vertical" => Transform::Flip(FlipAxis::Vertical),
                _ => bail!("flip must be \"horizontal\" or \"vertical\", got {axis:?}"),
            },
            RawStep::Crop {
                x,
                y,
                width,
                height,
            } => {
                ensure!(
                    width > 0 && height > 0,
                    "crop width and height must be positive"
                );
                Transform::Crop {
                    x,
                    y,
                    width,
                    height,
                }
            }
            RawStep::Resize { width, height } => {
                ensure!(
                    width > 0 && height > 0,
                    "resize width and height must be positive"
                );
                Transform::Resize { width, height }
            }
            RawStep::Brightness(offset) => {
                ensure!(
                    (-255..=255).contains(&offset),
                    "brightness must be in -255..=255, got {offset}"
                );
                Transform::Brightness(offset as i16)
            }
            RawStep::Contrast(factor) => {
                ensure!(
                    factor.is_finite() && factor >= 0.0,
                    "contrast must be a non-negative number, got {factor}"
                );
                Transform::Contrast(factor)
            }
            RawStep::Gamma(gamma) => {
                ensure!(
                    gamma.is_finite() && gamma > 0.0,
                    "gamma must be a positive number, got {gamma}"
                );
                Transform::Gamma(gamma)
            }
            RawStep::Grayscale => Transform::Grayscale,
        })
    }
}

/// Validated per-device pipelines.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PreprocessConfig {
    pub pipelines: HashMap<String, Pipeline>,
}

impl PreprocessConfig {
    /// Parses and validates a config. Invalid parameters and duplicate device ids are errors.
    pub fn from_toml(text: &str) -> Result<Self> {
        let raw: RawConfig = toml::from_str(text).context("invalid pre-processor config")?;
        let mut pipelines = HashMap::new();
        for device in raw.device {
            let steps = device
                .steps
                .into_iter()
                .enumerate()
                .map(|(i, step)| {
                    step.validate()
                        .with_context(|| format!("device {:?}, step {}", device.id, i + 1))
                })
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                pipelines
                    .insert(device.id.clone(), Pipeline { steps })
                    .is_none(),
                "device {:?} is configured twice",
                device.id
            );
        }
        Ok(PreprocessConfig { pipelines })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read pre-processor config {}", path.display()))?;
        Self::from_toml(&text).with_context(|| format!("in {}", path.display()))
    }
}

/// Applies a `PreprocessConfig` to frame packets.
#[derive(Debug, Clone)]
pub struct Preprocessor {
    config: PreprocessConfig,
}

impl Preprocessor {
    pub fn new(config: PreprocessConfig) -> Self {
        Preprocessor { config }
    }

    /// The device ids that have a pipeline.
    pub fn devices(&self) -> impl Iterator<Item = &str> {
        self.config.pipelines.keys().map(String::as_str)
    }

    /// Runs the device's pipeline on a camera frame. Returns `None` for frames from devices without a
    /// pipeline and for non-camera frames. The output carries `RGB3` or `GREY` with the new shape;
    /// device info, timestamps and everything else in the header are kept, so processed frames still
    /// align with the rest of the stream.
    pub fn process(&self, packet: &FramePacket) -> Result<Option<FramePacket>> {
        let info = &packet.frame_info;
        if info.device_info.device_type != DeviceType::Camera {
            return Ok(None);
        }
        let Some(pipeline) = self.config.pipelines.get(&info.device_info.id) else {
            return Ok(None);
        };
        let Some(format) = info.pixel_format.as_deref() else {
            bail!(
                "camera frame from {:?} carries no pixel format",
                info.device_info.id
            );
        };
        let image = pipeline.apply(decode_payload(&packet.data, format, &info.frame_shape)?)?;

        let mut out_info = info.clone();
        out_info.frame_shape = vec![image.width, image.height];
        out_info.pixel_format = Some(image.format.fourcc().to_string());
        Ok(Some(FramePacket::new(
            out_info,
            image.data.into_boxed_slice(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_step_kind() {
        let config = PreprocessConfig::from_toml(
            r#"
            [[device]]
            id = "cam"
            steps = [
              { rotate = 90 },
              { flip = "vertical" },
              { crop = { x = 1, y = 2, width = 3, height = 4 } },
              { resize = { width = 5, height = 6 } },
              { brightness = -10 },
              { contrast = 1.5 },
              { gamma = 0.8 },
              "grayscale",
            ]
            "#,
        )
        .unwrap();
        assert_eq!(
            config.pipelines["cam"].steps,
            vec![
                Transform::Rotate(Rotation::Deg90),
                Transform::Flip(FlipAxis::Vertical),
                Transform::Crop {
                    x: 1,
                    y: 2,
                    width: 3,
                    height: 4
                },
                Transform::Resize {
                    width: 5,
                    height: 6
                },
                Transform::Brightness(-10),
                Transform::Contrast(1.5),
                Transform::Gamma(0.8),
                Transform::Grayscale,
            ]
        );
    }

    #[test]
    fn empty_config_has_no_pipelines() {
        assert!(
            PreprocessConfig::from_toml("")
                .unwrap()
                .pipelines
                .is_empty()
        );
    }

    #[test]
    fn rejects_invalid_configs() {
        for bad in [
            r#"[[device]]
               id = "cam"
               steps = [{ rotate = 45 }]"#,
            r#"[[device]]
               id = "cam"
               steps = [{ flip = "diagonal" }]"#,
            r#"[[device]]
               id = "cam"
               steps = [{ brightness = 300 }]"#,
            r#"[[device]]
               id = "cam"
               steps = [{ gamma = 0.0 }]"#,
            r#"[[device]]
               id = "cam"
               steps = [{ resize = { width = 0, height = 1 } }]"#,
            r#"[[device]]
               id = "cam"
               steps = [{ sharpen = 1 }]"#,
            r#"[[device]]
               id = "cam"
               steps = []
               [[device]]
               id = "cam"
               steps = []"#,
        ] {
            assert!(PreprocessConfig::from_toml(bad).is_err(), "accepted: {bad}");
        }
    }
}
