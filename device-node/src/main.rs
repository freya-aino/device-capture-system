use anyhow::{Error, Result};
use device_node::{
    DeviceNode, get_all_devices, print_all_devices, test_alsa_mic, test_v4l_cameras,
};
use std::{env::consts::OS, os::unix::ffi::OsStringExt};
use warp::Filter;

#[tokio::main]
async fn main() {
    assert_eq!(OS, "linux", "Unsupported OS: {}", OS);

    // get all available devices
    let (mut all_cams, mut all_mics) = get_all_devices().unwrap();

    // TODO - filter out empty or placeholder devices

    print_all_devices(&all_cams, &all_mics);

    // let mut cam = all_cams.remove(0);
    // let mut mic = all_mics.remove(1);

    // test_v4l_cameras(&mut cam, 5)?;
    // test_alsa_mic(&mut mic, 5)?;

    // // TODO - establish HTTP Routes
    // let status = warp::path("/status");
    // let get_params = warp::path("/parameters")
    //     .and(warp::path::param())
    //     .map(|param: String| {
    //         match param {
    //             // process device id
    //         }
    //     });
    // // .and(warp::header(""))
    //
    // warp::serve(endpoint).run(([0, 0, 0, 0], 8080)).await;
}

// let dn = DeviceNode::new(cam, Ipv4Addr::from_str(ip_addr_str), port, 128, 1024);

//---

// // cli commands
// match Cli::parse().command {
//     ClapCommand::Init => {
//         println!("Initializing device node...");
//         println!("Creating cache directory...");
//         let mut home_path = std::env::var("HOME").unwrap();
//         home_path.push_str("/.cache/device-capture-device-node/");
//         let out = Command::new("mkdir")
//             .arg("-p")
//             .arg(home_path)
//             .output()
//             .unwrap();
//         println!(
//             "{}{}",
//             String::from_utf8_lossy(&out.stderr),
//             String::from_utf8_lossy(&out.stdout)
//         );

//         println!("Saving Configuration")
//     }
//     ClapCommand::List { verbose } => {
//         println!("--> {} camera devices found:", camera_dms.len());

//         for dm in camera_dms {
//             println!("{:?}", dm.device.device_info);

//             if verbose {}
//         }

//         println!("--> {} microphone devices found:", microphone_dms.len());

//         for dm in microphone_dms {
//             println!("{:?}", dm.device.device_info);

//             if verbose {}
//         }
//     }
//     ClapCommand::Run { host, port } => {
//         println!("Running device node on {}:{}...", host, port);
//     }
// }

// start_static_http_server("0.0.0.0", "8080", ())
