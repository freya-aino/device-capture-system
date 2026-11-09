//! Configurable basic image transforms, applied per camera in the hub or in a consumer.
//!
//! Pure Rust with no OpenCV, so `control-hub` can use it without linking OpenCV. It prepares images and
//! does no analysis.

mod config;
mod decode;
mod image;
mod transform;

pub use config::{PreprocessConfig, Preprocessor};
pub use decode::decode_payload;
pub use image::{Image, ImageFormat};
pub use transform::{FlipAxis, Pipeline, Rotation, Transform};
