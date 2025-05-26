

fn all_devices_paths_linux() -> Result<Vec<OsString>, Error> {
    use std::ffi::OsString;

    let device_paths = glob("/dev/video*")?
        .filter_map(Result::ok)
        .map(|path| path.into_os_string())
        .collect::<Vec<OsString>>();
    Ok(device_paths)
}


// device.rs
pub enum DeviceType {
    Camera,
    Microphone,
}

#[derive(Debug)]
pub struct DeviceInformation {
    pub id: u16,
    pub name: String,
    pub device_type: DeviceType,
}

impl DeviceInformation {
    pub fn new(id: u8, name: String, device_type: DeviceType) -> DeviceInformation {
        DeviceInformation {
            id,
            name,
            device_type,
        }
    }
}


#[derive(Debug)]
pub struct CameraConfig {
    width: u32,
    height: u32,
    fps: f32,
    fourcc: String,
}

#[derive(Debug)]
pub struct MicrophoneConfig {
    sample_rate: u32,
    channels: u8,
    sample_size: u16,
}