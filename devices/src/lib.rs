mod camera;
mod manager;

pub use camera::*;

#[cfg(target_os = "linux")]
mod microphone_linux {
    pub use microphone_linux::*;
}

#[cfg(target_os = "windows")]
mod microphone_windows {
    pub use microphone_windows::*;
}
