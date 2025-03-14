use anyhow::{Result, Error};
use nokhwa::query;
use nokhwa::native_api_backend;
use nokhwa::utils::FrameFormat;
use nokhwa::Camera;
use nokhwa::utils::CameraFormat;
use nokhwa::utils::CameraIndex;
use nokhwa::utils::RequestedFormat;
use nokhwa::utils::RequestedFormatType;
use nokhwa::pixel_format::RgbFormat;
use cpal::traits::{DeviceTrait, HostTrait};

// #[derive(Parser, Debug)]
// #[command(version, about, long_about = None)]
// struct Args {
//     #[arg(short = 'l', long = "list-cameras")]
//     list_cameras: bool,

//     #[arg(short, long)]
//     verbose: bool,
// }

// fn print_cameras_human_readable(cameras: &Vec<CameraInfo>, verbose: bool) -> Result<(), Error> {
    
//     println!("listing {} cameras\n", cameras.len());
    
//     for camera in cameras {
//         let index = camera.index().to_string().parse::<u8>().expect("failed to parse camera index");
//         let name = camera.human_name();
//         let device = camera.misc();
//         let description = camera.description();

//         if verbose {
//             println!("Index:       {:?}\nName:        {:?}\nDevice:      {:?}\nDescription: {:?}\n", index, name, device, description);
//         } else {
//             println!("Index: {:?}\nName:  {:?}\n", index, name);
//         }
//     }
//     Ok(())
// }

mod datamodel;
use datamodel::CameraDevice;

fn get_all_cameras() -> Result<Vec<CameraDevice>, Error> {
    let mut cameras = Vec::new();

    let backend = native_api_backend().unwrap();
    let camera_infos = query(backend).expect("failed to query cameras");
    
    for info in camera_infos {
        let camera = CameraDevice::new(
            info.index().clone(),
            info.human_name().clone(),
            None,
            None
        );
        
        cameras.push(camera);
    }
    Ok(cameras)
}

// fn get_all_camera_formats() -> Result<Vec<CameraFormat>, Error> {
//     let mut formats = Vec::new();
//     for camera_index in CameraIndex::iter() {
//         let camera_formats = get_camera_formats(&camera_index)?;
//         formats.extend(camera_formats);
//     }
//     Ok(formats)
// }


fn main() {

    // let args = Args::parse();

    // // list cameras
    // if args.list_cameras {
    //     print_cameras_human_readable(&camera_infos, args.verbose).unwrap();
    // }

    // list camera configuration options for one camera

    // list_camera_formats_human_readable(camera_infos[0].index()).unwrap();
    

    let host = cpal::default_host();
    let devices = host.devices().unwrap();

    for device in devices {
        
        let audio_input_config = device.supported_input_configs().unwrap();
        let name = device.name().unwrap();
        
        for config in audio_input_config{
            println!("I: {:?} : {:?}", name, config);
        }
    }
    
    let request_format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
    let camera_id = CameraIndex::Index(0);
    let mut cam = Camera::new(camera_id, request_format).unwrap();
    let mut formats = cam.compatible_camera_formats().unwrap();
    formats.sort_by_key(|a| a.width() * a.height() * a.frame_rate());

    for f in formats {
        println!("{:?}", f);
    }

}