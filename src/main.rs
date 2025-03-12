use anyhow::Result;
use anyhow::Error;
use core::panic;

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





fn main() {

    match nokhwa::nokhwa_check() {
        true => println!("Nokhwa is working"),
        false => panic!("Nokhwa is not working"),
    }


    let backend = native_api_backend().unwrap();

    println!("backend: {:?}", backend);



    let camera_list = query(backend).unwrap();

    for camera in camera_list {
        println!("{:?}", camera);
    }

    // let requested_format: RequestedFormat = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
    // let camera_index = CameraIndex::Index(0);
    // let cam = Camera::new(camera_index, requested_format).unwrap();
    // let cam_info = cam.info();



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
