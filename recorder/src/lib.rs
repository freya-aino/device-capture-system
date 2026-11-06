//! Records selected hub streams to disk: one video per camera, one WAV per microphone, and a
//! per-frame sidecar with the wire timestamps (`docs/step-24-recorder.md`).
//!
//! Scaffold: configuration, naming and the on-disk formats are implemented; capture and writing are
//! not.

mod config;
mod naming;
mod session;
mod utc;

pub use config::{Codec, RecorderConfig, Source};
pub use naming::{StemAllocator, file_stem, session_dir, session_id};
pub use session::{DeviceRecord, SessionInfo, SidecarLine};
pub use utc::{micros_since_epoch, rfc3339};
