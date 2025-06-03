use anyhow::Error;
use clap::{Parser, Subcommand};
use devices::V4lCameraDevice;
use glob::glob;
use std::{env::consts::OS, ffi::OsString, thread::sleep, time::Duration};
use v4l::FourCC;

use shared::{CameraConfig, Device, DeviceCommand};

#[derive(Parser)]
#[command(name = "device-node")]
struct Cli {
    #[command(subcommand)]
    command: ClapCommand,
}

#[derive(Subcommand)]
enum ClapCommand {
    Init,
    List {
        #[arg(short, long)]
        verbose: bool,
    },
    Run {
        #[arg(short, long, default_value_t = String::from("0.0.0.0"))]
        host: String,
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
}

// fn get_devices_for_card(card: Card) -> Result<Vec<DeviceInformation>, Error> {
//     let card_ctl = Ctl::from_card(&card, false)?;

//     let card_info = card_ctl.card_info()?;
//     let card_id = card_info.get_id()?;
//     let card_name = card_info.get_name()?;

//     let mut out_devices = Vec::<DeviceInformation>::new();

//     for device in DeviceIter::new(&card_ctl) {
//         let pcm_info = card_ctl.pcm_info(device as u32, 0, Direction::Capture);

//         match pcm_info {
//             Ok(info) => {
//                 for subdev_id in 0..info.get_subdevices_count() {
//                     let subdev_info =
//                         card_ctl.pcm_info(device as u32, subdev_id, Direction::Capture)?;

//                     // println!("Card name: {:?}", card_name);
//                     // println!("Card id: {:?}", card_id);
//                     // println!("Device name: {:?}", device_name);

//                     out_devices.push(DeviceInformation {
//                         id: device as u16,
//                         name: card_name.to_string(),
//                         device_type: DeviceType::Microphone,
//                     })
//                 }
//             }
//             Err(_) => continue,
//         }
//     }

//     Ok(out_devices)
// }

fn main() -> Result<(), Error> {
    assert_eq!(OS, "linux", "Unsupported OS: {}", OS);

    let mut all_cameras = V4lCameraDevice::get_all_v4l_devices()?;

    println!("num cameras: {:?}", all_cameras.len());
    for cam in all_cameras.iter() {
        println!("Camera: {:?}", cam.name());
    }

    let mut cam = all_cameras.remove(0);
    let mut all_configs = cam.get_configs()?;

    for (i, cc) in all_configs.iter().enumerate() {
        println!("Config {}: {:?}", i, cc);
    }

    let config = CameraConfig {
        width: 1920,
        height: 1080,
        fps: (1, 60),
        fourcc: FourCC::new(b"YUYV").repr,
    };

    let handle = cam.start(
        config.clone(),
        Box::new(|a| {
            println!("data len: {:?}", a.data.len());
        }),
    )?;

    println!("Camera started with config: {:?}", config);

    sleep(Duration::from_secs(3));

    cam.stop()?;

    handle.join().unwrap();

    // let cards = Iter::new()
    //     .filter_map(Result::ok)
    //     .map(|c| get_devices_for_card(c).unwrap())
    //     .flatten()
    //     .collect::<Vec<DeviceInformation>>();

    // for card in cards {
    //     println!("Card: {:?}", card);
    // }
    //

    // let hints = HintIter::new_str(None, "pcm")?;
    // for hint in hints {
    //     if !hint.direction.is_none() || hint.direction == Some(Direction::Playback) {
    //         continue;
    //     }

    //     out_device_infos.push(DeviceInformation {
    //         id: hint.name.unwrap(),
    //         name: hint.desc.unwrap(),
    //     })
    // }

    // for device_info in out_device_infos {
    //     println!("Device: {:?}", device_info);
    // }

    // for card in cards {

    // // let capture = alsa::pcm::
    // let pcm = PCM::new(&card_info.get_name()?, Direction::Capture, false)?;

    // let pcm_info = pcm.info()?;
    // println!("PCM: {:?}", pcm_info.get_name());

    // let params = HwParams::any(&pcm)?;

    // let min_buffer = params.get_buffer_size_min()?;
    // let max_buffer = params.get_buffer_size_max()?;
    // let min_period = params.get_period_size_min()?;
    // let max_period = params.get_period_size_max()?;
    // let min_channels = params.get_channels_min();
    // let max_channels = params.get_channels_max();
    // let format = params.get_format()?;
    // // params.test_format()

    // println!("min_buffer: {:?}", min_buffer);
    // println!("max_buffer: {:?}", max_buffer);
    // println!("min_period: {:?}", min_period);
    // println!("max_period: {:?}", max_period);
    // println!("min_channels: {:?}", min_channels);
    // println!("max_channels: {:?}", max_channels);
    // println!("format: {:?}", format);

    // // get all devices
    // let cpal_host = cpal::default_host();
    // let camera_dms = DeviceManager::<V4lCameraDevice>::get_all_devices().unwrap();
    // let microphone_dms =
    //     DeviceManager::<CpalMicrophoneDevice>::get_all_devices(&cpal_host).unwrap();

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

    Ok(())
}
