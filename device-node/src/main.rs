use devices::{get_all_camera_devices_linux, get_all_microphone_devices};

fn main() {
    let cpal_host = cpal::default_host();

    // get all devices
    let camera_devices = get_all_camera_devices_linux().unwrap();
    let microphone_devices = get_all_microphone_devices(&cpal_host).unwrap();

    println!("--> {} camera devices found:", camera_devices.len());
    println!("--> {} microphone devices found:", microphone_devices.len());

    for device in camera_devices {
        println!("{:?}", device.device_info);
    }
    for device in microphone_devices {
        println!("{:?}", device.device_info);
    }
    // get all configurations for all devices
    // open a http server to advertise the device control
    //
}
