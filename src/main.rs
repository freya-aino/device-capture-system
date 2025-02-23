use anyhow::Result;
use core::panic;

// use nokhwa::Camera;
// use nokhwa::pixel_format::RgbFormat;
// use nokhwa::utils::CameraIndex;
// use nokhwa::utils::RequestedFormat;
// use nokhwa::utils::RequestedFormatType;

#[derive(Debug, PartialEq)]
enum OperatingSystem {
    Windows,
    Linux,
    MacOS,
}

fn get_current_os() -> OperatingSystem {
    match std::env::consts::OS {
        "windows" => OperatingSystem::Windows,
        "linux" => OperatingSystem::Linux,
        "macos" => OperatingSystem::MacOS,
        _ => panic!("Unsupported OS"),
    }
}

fn v4l2_exists() -> bool {
    let res = std::process::Command::new("v4l2-ctl")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match res {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[derive(Debug)]
struct V4l2Device {
    name: String,
}

fn v4l2_get_device_list() -> Result<Vec<V4l2Device>> {
    let video_devices = v4l::context::enum_devices();
    let mut devices: Vec<V4l2Device> = Vec::new();

    for device in video_devices {
        let d = V4l2Device {
            name: device.name().unwrap(),
        };

        devices.push(d);
    }

    Ok(devices)
}

fn main() {
    match get_current_os() {
        OperatingSystem::Linux => println!("Running on Linux"),
        OperatingSystem::Windows => panic!("Windows not supported"),
        OperatingSystem::MacOS => panic!("MacOS not supported"),
    }

    match nokhwa::nokhwa_check() {
        true => println!("Nokhwa is working"),
        false => panic!("Nokhwa is not working"),
    }

    match v4l2_exists() {
        true => println!("v4l2 exists"),
        false => panic!("v4l2 does not exist"),
    }

    let backend = nokhwa::native_api_backend().unwrap_or_else(|| panic!("No backend found"));

    println!("backend: {:?}", backend);

    let device_list = v4l2_get_device_list();

    println!("number of devices: {}", Vec::len(device_list.as_ref()));

    {
        for device in device_list.unwrap() {
            println!("device: {:?}", device.name);
        }
    }

    // {
    //     let mut cam = Camera::new(
    //         CameraIndex::Index(0),
    //         RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
    //     )
    //     .unwrap();

    //     let frame = cam.frame().unwrap();

    //     println!("{:?}", frame.buffer().len());
    // }
    // nokhwa::query(backend)
    //     .unwrap_or_else(|e| panic!("Error querying cameras: {}", e));

    // let cameras= nokhwa::query(ApiBackend::Auto)
    //     .unwrap_or_else(|e| panic!("Error querying cameras: {}", e));

    // for camera in cameras {
    //     println!("{:?}", camera);
    // }
}
