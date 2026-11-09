use devices::V4lCameraDevice;
use shared::{Device, DeviceStatus, DeviceType};

#[test]
fn opening_a_missing_camera_path_returns_error() {
    let result = V4lCameraDevice::new("/dev/device-capture-system-missing-camera".to_string());
    assert!(result.is_err());
}

#[test]
fn enumerating_cameras_does_not_fail_without_hardware() {
    // Skips unusable /dev/video* nodes with a warning instead of failing,
    // so enumeration succeeds whether or not a camera is attached.
    let cameras = V4lCameraDevice::get_all_v4l_devices();
    assert!(cameras.is_ok());
}

#[test]
#[ignore = "requires a real V4L2 capture device at /dev/video0"]
fn real_camera_reports_camera_identity_and_configs() {
    let camera = V4lCameraDevice::new("/dev/video0".to_string())
        .expect("a V4L2 capture device should be present at /dev/video0");

    assert_eq!(camera.id(), "/dev/video0");
    assert!(!camera.name().is_empty());
    assert_eq!(camera.device_type(), &DeviceType::Camera);
    assert_eq!(camera.device_status(), &DeviceStatus::Available);

    let configs = camera.get_configs().unwrap();
    assert!(!configs.is_empty());
}
