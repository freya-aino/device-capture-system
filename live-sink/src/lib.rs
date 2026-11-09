//! Writes one selected hub stream into a `v4l2loopback` device, so applications that only open a
//! camera can use it live (`docs/step-25-live-output.md`).

mod config;
mod convert;
mod format;
mod forward;
mod sink;

pub use config::{LiveConfig, LiveSource};
pub use convert::{convert_frame, grey_to_yuyv, rgb_to_yuyv};
pub use format::{Conversion, OutputFormat, OutputPlan, plan_output};
pub use forward::{ForwardStats, forward, forward_with};
pub use sink::{FrameSink, V4l2Output};
