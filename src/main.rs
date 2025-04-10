mod devices;
mod clap_commands;


use std::sync::Arc;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use clap::Parser;
use clap_commands::{Cli, Commands, DeviceCommand};

// use nokhwa::utils::CameraIndex;
// use nokhwa::pixel_format::{RgbAFormat, RgbFormat};


// fn start_cam_thread(device: Device) -> Result<(), Error> {

//     let cam_unwrap = cam?;

// //     for camera in cameras {
// //         let index = camera.index().to_string().parse::<u8>().expect("failed to parse camera index");
// //         let name = camera.human_name();
// //         let device = camera.misc();
// //         let description = camera.description();

// //         if verbose {
// //             println!("Index:       {:?}\nName:        {:?}\nDevice:      {:?}\nDescription: {:?}\n", index, name, device, description);
// //         } else {
// //             println!("Index: {:?}\nName:  {:?}\n", index, name);
// //         }
// //     }
// //     Ok(())
// // }

// fn get_all_cameras() -> Result<Vec<CameraDevice>, Error> {
//     let mut cameras: Vec<CameraDevice> = Vec::new();

//     let backend = native_api_backend().unwrap();
//     let camera_infos = query(backend).expect("failed to query cameras");

//     for info in camera_infos {

//         // let device string be misc field if it is not empty
//         let misc = match info.misc() {
//             s if s.is_empty() => None,
//             s => Some(s.to_string()),
//         };

//         let camera = CameraDevice::new(
//             info.index().as_index().unwrap() as u16,
//             info.human_name().to_string(),
//             misc,
//         );

//         cameras.push(camera);
//     }
//     Ok(cameras)
// }

// fn get_all_microphones() -> Result<Vec<MicrophoneDevice>, Error> {
//     let mut microphones: Vec<MicrophoneDevice> = Vec::new();

//     let host = cpal::default_host();
//     let devices = host.input_devices().unwrap();
//     for (i, device) in devices.enumerate() {

//         let configs = device.supported_input_configs().unwrap();

//         for config in configs {
//             println!("{:?}", config);
//         }

//         let microphone = MicrophoneDevice::new(
//             i as u16,
//             device.name().unwrap(),
//             None,
//         );

//         microphones.push(microphone);
//     }
//     Ok(microphones)
// }


// use devices::DeviceManager;
// use devices::MicrophoneDevice;
// use devices::MicrophoneConfig;

use anyhow::{Error, Result};

use clap::{Parser, ValueEnum};

#[tokio::main]
async fn main() -> Result<(), Error> {

// --------- functions --------- //

fn get_audio_devices() -> Result<Vec<Microphone>, Error> {
    let host = cpal::default_host();
    let devices = host.input_devices()
        .unwrap()
        .collect::<Vec<Microphone>>();
    Ok(devices)
}

// ---------- main ---------- //

use anyhow::Error;
use datamodel::Device;
use nokhwa::Camera;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

fn main() {
    let cam_infos = Device::get_all_cameras().unwrap();

    println!("cameras : {:?}", cam_infos);

    let backend = nokhwa::native_api_backend().unwrap();

    let cam_infos = nokhwa::query(backend).unwrap();


        let cam = Camera::new(
            CameraIndex::Index(info.id.0 as u32),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
        )
        .unwrap();

        // for format in formats {
        //     out_configs.push(
        //         CameraConfig {
        //             resolution: (format.resolution().width(), format.resolution().height()),
        //             fps: format.frame_rate(),
        //             pixel_format: format.format(),
        //         }
        //     );
        // }

        // println!("device created ---")
    }

    // let camera_infos = nokhwa::query(backend).expect("failed to query cameras");

    // for info in camera_infos {
    //     println!("{:?}", info.index().as_index().unwrap() as u32);
    // }

    // let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    // let mut cam = Camera::new(
    //     CameraIndex::Index(1),
    //     format
    // ).unwrap();

    // cam.stop_stream().unwrap();

    // let backend = nokhwa::native_api_backend().unwrap();
    // let camera_infos = nokhwa::query(backend).expect("failed to query cameras");

    println!("{:?}", format);

    let device_0 = Device::new(0);
}
