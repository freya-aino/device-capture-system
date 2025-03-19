// mod datamodel;

// use anyhow::{Result, Error};
// use datamodel::{Device, CameraDevice, MicrophoneDevice};
// use nokhwa::query;
// use nokhwa::native_api_backend;
// // use nokhwa::utils::FrameFormat;
// // use nokhwa::Camera;
// // use nokhwa::utils::CameraFormat;
// // use nokhwa::utils::CameraIndex;
// // use nokhwa::utils::RequestedFormat;
// // use nokhwa::utils::RequestedFormatType;
// // use nokhwa::pixel_format::RgbFormat;
// use cpal::traits::{DeviceTrait, HostTrait};

// // #[derive(Parser, Debug)]
// // #[command(version, about, long_about = None)]
// // struct Args {
// //     #[arg(short = 'l', long = "list-cameras")]
// //     list_cameras: bool,

// //     #[arg(short, long)]
// //     verbose: bool,
// // }

// // fn print_cameras_human_readable(cameras: &Vec<CameraInfo>, verbose: bool) -> Result<(), Error> {
    
// //     println!("listing {} cameras\n", cameras.len());
    
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



// // fn get_all_microphone_formats() -> Result<Vec<MicrophoneConfig>, Error>


// fn main() {

//     // let args = Args::parse();

//     // // list cameras
//     // if args.list_cameras {
//     //     print_cameras_human_readable(&camera_infos, args.verbose).unwrap();
//     // }

//     // list camera configuration options for one camera

//     // list_camera_formats_human_readable(camera_infos[0].index()).unwrap();
    
//     let cameras = get_all_cameras().unwrap();
//     let microphones = get_all_microphones().unwrap();

//     for camera in cameras {
//         println!("{:?}", camera);
//     }
//     for microphone in microphones {
//         println!("{:?}", microphone);
//     }

//     // let host = cpal::default_host();
//     // let devices = host.devices().unwrap();

//     // for device in devices {
        
//     //     let audio_input_config = device.supported_input_configs().unwrap();
//     //     let name = device.name().unwrap();
        
//     //     for config in audio_input_config{
//     //         println!("I: {:?} : {:?}", name, config);
//     //     }
//     // }
    
//     // let camera_id = CameraIndex::Index(0);
//     // let mut cam = Camera::new(camera_id, request_format).unwrap();
//     // let mut formats = cam.compatible_camera_formats().unwrap();
//     // formats.sort_by_key(|a| a.width() * a.height() * a.frame_rate());

//     // for f in formats {
//     //     println!("{:?}", f);
//     // }

// }


mod datamodel;

use datamodel::Device;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::Camera;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

fn main() {
    let cam_infos = Device::get_all_cameras().unwrap();

    println!("cameras : {:?}", cam_infos);

    let backend = nokhwa::native_api_backend().unwrap();

    let cam_infos = nokhwa::query(backend);

    for info in cam_infos.unwrap() {
        println!("outside : {:?}", info);

        // let dev = Device::new(info);

        let cam = Camera::new(
            info.index().clone(),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
        );

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


    // for ci in camera_infos {
    //     println!("{:?}", ci.index().as_index().unwrap() as u32);

    // }
}