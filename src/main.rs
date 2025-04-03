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

use clap::{Parser, ValueEnum};

// --------- structs --------- //

#[derive(Debug, Clone, ValueEnum, PartialEq)]
enum DeviceType {
    Camera,
    Microphone,
}

// ---------- main function --------- //

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {

    /// list devices
    #[clap(short, long)]
    list_devices: bool,

    // start capture, device_type and index are required
    #[clap(short, long)]
    capture: bool,

    // device type to capture
    #[clap(short, long, value_enum)]
    device_type: Option<DeviceType>,

    // device index to capture, to find index run `--list-devices`
    #[clap(short, long)]
    index: Option<usize>,
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
        .collect::<Vec<Microphone>>();
    Ok(devices)
}

fn main() -> Result<(), Error> {

    let args = Args::parse();

    let camera_devices = get_camera_devices()?;
    let audio_devices = get_audio_devices()?;

    // list devices
    if args.list_devices {
        println!("Listing devices...");
        // list_devices()?

        println!("\nCamera Devices Found: {}", camera_devices.len());
        for dev in camera_devices.iter() {
            println!("{}:\t: {}", dev.index(), dev.human_name());
        }
        println!("\nAudio Devices Found: {}", audio_devices.len());
        for (i, dev) in audio_devices.iter().enumerate() {
            println!("{}:\t: {}", i, dev.name()?);
        }
    }

    // select device to use
    if args.capture {
        
        // parse index option
        let index = match args.index {
            Some(index_) => index_,
            None => {
                println!("No device index selected, select device index to capture. device index can be found using `--list-devices`");
                return Ok(());
            }
        };

        match args.device_type {
            Some(DeviceType::Camera) => {

                // check camera index exists
                if camera_devices
                    .iter()
                    .find(|dev| dev.index().as_index().unwrap() as usize == index)
                    .is_none() {
                    println!("Invalid camera index: {}", index);
                    return Ok(());
                }

                // start camera
                println!("try start capture - camera: {}", camera_devices[index].human_name());

            },
            Some(DeviceType::Microphone) => {

                // check microphone index exists
                match args.index {
                    Some(index_) => {
                        if index_ >= audio_devices.len() {
                            println!("Invalid microphone index: {}", index_);
                            return Ok(());
                        }

                        println!("Selected microphone: {}", audio_devices[index_].name()?);
                    },
                    None => {
                        println!("No device index selected, please select a device index to capture.");
                        return Ok(());
                    }
                };

                // start capture

                // start microphone
                println!("try start capture - microphone: {}", audio_devices[index].name()?);
            },
            None => {
                println!("No device type selected, please select a device type to capture.");
                return Ok(());
            }
        }
    }




    Ok(())

    // let camera = nokhwa::Camera::new(
    //     CameraIndex::Index(0),
    //     RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
    // );
    
}
