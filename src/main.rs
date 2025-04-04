mod devices;

use devices::{CameraDevice, Device};

use std::thread::sleep;
use std::time;

use anyhow::{Error, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device as Microphone, InputCallbackInfo};
use nokhwa::utils::{CameraIndex, CameraInfo, RequestedFormat, RequestedFormatType};
use nokhwa::pixel_format::RgbFormat;

use clap::{Parser, ValueEnum};


// --------- functions --------- //

fn get_audio_devices() -> Result<Vec<Microphone>, Error> {
    let host = cpal::default_host();
    let devices = host.input_devices()
        .unwrap()
        .collect::<Vec<Microphone>>();
    Ok(devices)
}

// ---------- main ---------- //

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


fn main() -> Result<(), Error> {

    let args = Args::parse();

    // let camera_devices = get_camera_devices()?;
    
    let cameras = match CameraDevice::get_all_available_devices() {
        Ok(cameras) => cameras,
        Err(e) => {
            println!("Failed to get camera devices: {}", e);
            return Ok(());
        }
    };

    for cam in cameras.iter() {
        println!("{:?}", cam.ID);
    }

    // let audio_devices = get_audio_devices()?;

    // // list devices
    // if args.list_devices {
    //     println!("Listing devices...");
    //     // list_devices()?

    //     println!("\nCamera Devices Found: {}", camera_devices.len());
    //     for dev in camera_devices.iter() {
    //         println!("{}:\t: {}", dev.index(), dev.human_name());
    //     }
    //     println!("\nAudio Devices Found: {}", audio_devices.len());
    //     for (i, dev) in audio_devices.iter().enumerate() {
    //         println!("{}:\t: {}", i, dev.name()?);
    //     }
    // }

    // // select device to use
    // if args.capture {
        
    //     // parse index option
    //     let index = match args.index {
    //         Some(index_) => index_,
    //         None => {
    //             println!("No device index selected, select device index to capture. device index can be found using `--list-devices`");
    //             return Ok(());
    //         }
    //     };

    //     match args.device_type {
    //         Some(DeviceType::Camera) => {

    //             // check camera index exists
    //             if camera_devices
    //                 .iter()
    //                 .find(|dev| dev.index().as_index().unwrap() as usize == index)
    //                 .is_none() {
    //                     println!("Invalid camera index: {}", index);
    //                     return Ok(());
    //                 }

    //             // start camera
    //             println!("try start capture - camera: {}", camera_devices[index].human_name());
    //             let mut cam = nokhwa::Camera::new(
    //                 CameraIndex::Index(index as u32),
    //                 RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
    //             )?;
    //             match cam.open_stream() {
    //                 Ok(_) => {
    //                     println!("Camera stream opened successfully.");
    //                 },
    //                 Err(e) => {
    //                     println!("Failed to open camera stream: {}", e);
    //                     return Ok(());
    //                 }
    //             }

    //             for i in 0..10 {
    //                 let frame = match cam.frame() {
    //                     Ok(frame) => frame,
    //                     Err(e) => {
    //                         println!("Failed to get frame: {}", e);
    //                         break;
    //                     }
    //                 };
    //                 println!("{:?}", frame.buffer_bytes().len());
                    
    //                 sleep(time::Duration::from_millis(200));
    //             }

    //             match cam.stop_stream() {
    //                 Ok(_) => {
    //                     println!("Camera stream stopped successfully.");
    //                 },
    //                 Err(e) => {
    //                     println!("Failed to stop camera stream: {}", e);
    //                     return Ok(());
    //                 }
    //             }
    //         },
    //         Some(DeviceType::Microphone) => {

    //             // check microphone index exists
    //             if index >= audio_devices.len() {
    //                 println!("Invalid microphone index: {}", index);
    //                 return Ok(());
    //             };

    //             // start capture

    //             // start microphone
    //             println!("try start capture - microphone: {}", audio_devices[index].name()?);

    //             let cfg = audio_devices[index]
    //                 .default_input_config()
    //                 .expect("failud to get default input config")
    //                 .config();

    //             let device_cfg = audio_devices[index]
    //                 .build_input_stream(
    //                     &cfg,
    //                     |data: &[f32], _: &InputCallbackInfo| {
    //                         // process audio data here
    //                         println!("Audio data: {:?}", data);
    //                     },
    //                     |err| {
    //                         // handle error here
    //                         eprintln!("Error: {:?}", err);
    //                     },
    //                     Some(time::Duration::from_secs(5)),
    //                 )?;
                
    //             device_cfg.play()?;
    //             sleep(time::Duration::from_secs(5));
    //             device_cfg.pause()?;

    //         },
    //         None => {
    //             println!("No device type selected, please select a device type to capture.");
    //             return Ok(());
    //         }
    //     }
    // }


    Ok(())

    // let camera = nokhwa::Camera::new(
    //     CameraIndex::Index(0),
    //     RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
    // );
    
}
