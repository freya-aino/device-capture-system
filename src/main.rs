use anyhow::{Result, Error};

use clap::Parser;
use core::panic;

use nokhwa::utils::ApiBackend;
use nokhwa::Camera;
use nokhwa::utils::CameraIndex;
use nokhwa::utils::CameraInfo;
use nokhwa::utils::RequestedFormat;
use nokhwa::utils::RequestedFormatType;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::query;
use nokhwa::native_api_backend;
// use nokhwa::pixel_format::RgbFormat;
// use nokhwa::utils::CameraIndex;
// use nokhwa::utils::RequestedFormat;
// use nokhwa::utils::RequestedFormatType;



#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 'l', long = "list-cameras")]
    list_cameras: bool,

    #[arg(short, long)]
    verbose: bool,
}


fn print_cameras_human_readable(cameras: &Vec<CameraInfo>, verbose: bool) -> Result<(), Error> {
    
    println!("listing {} cameras\n", cameras.len());
    
    for camera in cameras {
        let index = camera.index().to_string().parse::<u8>().expect("failed to parse camera index");
        let name = camera.human_name();
        let device = camera.misc();
        let description = camera.description();

        if verbose {
            println!("Index:       {:?}\nName:        {:?}\nDevice:      {:?}\nDescription: {:?}\n", index, name, device, description);
        } else {
            println!("Index: {:?}\nName:  {:?}\n", index, name);
        }
    }
    Ok(())
}


fn main() {

    match nokhwa::nokhwa_check() {
        true => println!("Nokhwa is working"),
        false => panic!("Nokhwa is not working"),
    }

    let args = Args::parse();
    let backend = native_api_backend().unwrap();
    let cameras = query(backend).expect("failed to query cameras");

    // list cameras
    if args.list_cameras {
        print_cameras_human_readable(&cameras, args.verbose).unwrap();
    }

    // list camera configuration options for one camera
    // let camera_index = cameras[0].index();
    
    


    // let requested_format: RequestedFormat = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
    // let camera_index = CameraIndex::Index(0);
    // let cam = Camera::new(camera_index, requested_format).unwrap();
}