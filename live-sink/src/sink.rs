//! The V4L2 output device (`v4l2loopback`): format set once, frames written with `write()`.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use v4l::video::Output;
use v4l::{Device, Format, FourCC};

use crate::OutputPlan;

/// Anything converted frames can be written to. `V4l2Output` in production; tests substitute a
/// buffer. The bytes are already in the plan's output format.
pub trait FrameSink {
    fn write_frame(&mut self, data: &[u8]) -> Result<()>;
}

pub struct V4l2Output {
    device: Device,
    plan: OutputPlan,
}

impl V4l2Output {
    /// Opens the loopback device and sets its output format from `plan`. The driver must accept
    /// exactly the requested size and FourCC.
    pub fn open(path: &Path, plan: OutputPlan) -> Result<Self> {
        let device = Device::with_path(path)
            .with_context(|| format!("opening {} for output", path.display()))?;
        let fourcc = FourCC::new(
            plan.output
                .fourcc()
                .as_bytes()
                .try_into()
                .expect("FourCC is 4 bytes"),
        );
        let wanted = Format::new(plan.width, plan.height, fourcc);
        let got = Output::set_format(&device, &wanted)
            .with_context(|| format!("setting the output format of {}", path.display()))?;
        ensure!(
            got.width == plan.width && got.height == plan.height && got.fourcc == fourcc,
            "{} accepted {}x{} {} instead of {}x{} {}",
            path.display(),
            got.width,
            got.height,
            got.fourcc,
            plan.width,
            plan.height,
            fourcc
        );
        Ok(V4l2Output { device, plan })
    }

    pub fn plan(&self) -> &OutputPlan {
        &self.plan
    }
}

impl FrameSink for V4l2Output {
    fn write_frame(&mut self, data: &[u8]) -> Result<()> {
        self.device
            .write_all(data)
            .context("writing a frame to the V4L2 output device")
    }
}
