
mod primitives;

#[cfg(feature = "device")]
mod camera;
#[cfg(feature = "device")]
pub use camera::V4lCameraDevice;

#[cfg(feature = "device")]
mod microphone;
#[cfg(feature = "device")]
pub use microphone::*;