use devices::AlsaMicrophoneDevice;
use shared::{Device, DeviceStatus, DeviceType};

#[test]
fn new_device_starts_available_and_untyped_as_microphone() {
    let mic = AlsaMicrophoneDevice::new("hw:0,0".to_string(), "Test Mic".to_string());

    assert_eq!(mic.id(), "hw:0,0");
    assert_eq!(mic.name(), "Test Mic");
    assert_eq!(mic.device_type(), &DeviceType::Microphone);
    assert_eq!(mic.device_status(), &DeviceStatus::Available);
}

#[test]
fn stop_before_start_returns_an_error_instead_of_panicking() {
    let mut mic = AlsaMicrophoneDevice::new("hw:0,0".to_string(), "Test Mic".to_string());

    assert!(mic.stop().is_err());
    // Stopping before starting must not change the reported status.
    assert_eq!(mic.device_status(), &DeviceStatus::Available);
}

#[test]
fn get_all_alsa_microphones_enumerates_without_hardware() {
    // This only exercises ALSA hint enumeration, which succeeds (possibly with an
    // empty list) whether or not a sound card is present.
    let microphones = AlsaMicrophoneDevice::get_all_alsa_microphones();
    assert!(microphones.is_ok());
}
