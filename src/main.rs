use std::sync::Arc;

// use anyhow::{Error, Result};

// use std::thread;
// use std::time::Duration;

// use nokhwa::utils::CameraIndex;
// use nokhwa::pixel_format::{RgbAFormat, RgbFormat};


// fn start_cam_thread(device: Device) -> Result<(), Error> {

//     let cam_unwrap = cam?;

//     return Ok(());
// }


// use devices::DeviceManager;
// use devices::MicrophoneDevice;
// use devices::MicrophoneConfig;

use anyhow::{Error, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Device as Microphone;
use nokhwa::utils::{CameraIndex, CameraInfo, RequestedFormat, RequestedFormatType};
use nokhwa::pixel_format::RgbFormat;

use clap::Parser;


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {

    /// list devices
    #[clap(short, long)]
    list_devices: bool,
}


fn get_camera_devices() -> Result<Vec<CameraInfo>, Error> {
    let nokhwa_backend = nokhwa::native_api_backend().unwrap();
    let devices = nokhwa::query(nokhwa_backend)
            .unwrap()
            .into_iter()
            .collect::<Vec<CameraInfo>>();
    Ok(devices)
}

fn get_audio_devices() -> Result<Vec<Microphone>, Error> {
    let host = cpal::default_host();
    let devices = host.input_devices()
        .unwrap()
        // .enumerate()
        // .map(|(i, device)| {
        //     let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        //     let index = i as u32;
        // })
        .collect::<Vec<Microphone>>();
    Ok(devices)
}


fn main() -> Result<(), Error> {

    let args = Args::parse();

    if args.list_devices {
        println!("Listing devices...");
        // list_devices()?

        let camera_devices = get_camera_devices()?;
        let audio_devices = get_audio_devices()?;

        println!("\nCamera Devices Found: {}", camera_devices.len());
        for dev in camera_devices.iter() {
            println!("{}:\t: {}", dev.index(), dev.human_name());
        }
        println!("\nAudio Devices Found: {}", audio_devices.len());
        for (i, dev) in audio_devices.iter().enumerate() {
            println!("{}:\t: {}", i, dev.name()?);
        }
    } else {
        println!("No devices listed.");
    }


    Ok(())

    // let camera = nokhwa::Camera::new(
    //     CameraIndex::Index(0),
    //     RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
    // );
    
}
