mod camera;

pub use camera::*;

// Linux only: the camera is v4l2 and the microphone is ALSA. Windows is not supported (README,
// Requirements; docs/specs step 18); a Windows microphone would need a WASAPI implementation of
// `shared::Device` and a camera backend to go with it.
mod microphone_linux;
pub use microphone_linux::*;
