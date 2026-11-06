//! Writes one selected hub stream into a `v4l2loopback` device, so applications that only open a
//! camera can use it live (`docs/step-25-live-output.md`).
//!
//! Scaffold: configuration and the output-format decision are implemented; the V4L2 writer is not.

mod config;
mod format;

pub use config::{LiveConfig, LiveSource};
pub use format::{Conversion, OutputFormat, OutputPlan, plan_output};
