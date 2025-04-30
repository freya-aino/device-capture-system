use anyhow::{Error, Result};
use clap::{Parser, Subcommand};
use glob::glob;
use std::ffi::OsString;
use std::net::Ipv4Addr;
use std::path::Path;
use std::thread::{self, spawn};
use std::time::{self, Duration};

use device_capture_system::{CpalMicrophoneDevice, FramePacket, Receiver, Sender};

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, PartialEq)]
pub enum Commands {
    // get all devices, or a specific device by index returns the device information
    Info {
        // device command
        #[clap(short, long)]
        index: Option<u8>,
    },
    // Device {

    //     #[command(subcommand)]
    //     device_command: DeviceCommand,
    // },
}

#[derive(Subcommand, PartialEq)]
pub enum DeviceCommand {
    Configs {
        #[command(subcommand)]
        config_command: ConfigCommand,
    },
    // Info,
    // Start,
    // Stop,
    // Pause,
    // Resume,
    // Terminate,
}

#[derive(Subcommand, PartialEq)]
pub enum ConfigCommand {
    list,
    set,
    get,
}

fn benchmark_sender(iterations: u32, data_size: u32, data_chunk_size: u32) -> Result<(), Error> {
    print!("Starting benchmark sender...");

    let mut sender = Sender::new(
        Ipv4Addr::new(127, 0, 0, 1),
        portpicker::pick_unused_port().unwrap(),
        10,
    );

    let context = zmq::Context::new();
    sender.initialize(&context).unwrap();

    let mut average_duration: Duration = Duration::new(0, 0);
    // time sending frames
    for _ in 0..iterations {
        let frame = vec![0u8; data_size as usize];

        let start_time = time::SystemTime::now();

        let fp = FramePacket::new(time::SystemTime::now(), vec![10, 20, 30], frame);
        sender.send(fp, zmq::DONTWAIT, data_chunk_size).unwrap();

        let end_time = time::SystemTime::now();

        let duration = end_time.duration_since(start_time).unwrap();
        average_duration += duration;
    }

    println!("Benchmark completed!");
    println!(
        "Average duration: {:?} -- Average fps: {:?}",
        average_duration.as_secs_f32() / (iterations as f32),
        1.0 / (average_duration.as_secs_f32() / (iterations as f32))
    );

    Ok(())
}

fn main() {
    // let device_paths = all_devices_paths_linux().unwrap();
    // println!("Found {} devices!", device_paths.len());
    // if device_paths.len() == 0 {
    //     println!("No devices found!");
    //     return;
    // }
    // let dev_path = device_paths.get(1).unwrap();
    // let dev = Device::with_path(dev_path).unwrap();
    // let caps = dev.query_caps().unwrap();
    // println!("{:?}", caps);

    let cpal_host = cpal::default_host();
    let mut mic = CpalMicrophoneDevice::new(0, &cpal_host);

    mic.print_configurations();
    println!("mic: {:?}", mic.get_name());

    mic.open(None, None, None, None, None).unwrap();

    thread::sleep(Duration::from_secs(1));

    mic.close().unwrap();

    // // get all devices
    // let all_device_managers = DeviceManager::get_all_available_devices(&nokhwa_backend, &cpal_host).unwrap();

    // match cli.command {
    //     Commands::Info { index } => {
    //         match index {
    //             Some(index) => {
    //                 if index as usize >= all_device_managers.len() {
    //                     println!("Invalid index: {} only {} devices available!", index, all_device_managers.len());
    //                     return;
    //                 }

    //                 let device_manager = all_device_managers.get(index as usize).unwrap();

    //                 match device_manager.device_info.device_type {
    //                     DeviceType::Camera => {
    //                         let mut cam = CameraDevice::new(
    //                             device_manager.device_info.id,
    //                             device_manager.device_info.name.clone(),
    //                         );
    //                         cam.initialize().unwrap();

    //                         println!("Device:      {:?}", device_manager.device_info.name);

    //                         let all_configs = cam.get_all_available_configs().unwrap();

    //                         println!("Configs Found: {}", all_configs.len());
    //                         for conf in all_configs.iter() {
    //                             println!("{:?}", conf);
    //                         }
    //                         println!("");

    //                     }
    //                     DeviceType::Microphone => {
    //                         println!("Microphone device found!");
    //                     }
    //                 }
    //             },
    //             None => {
    //                 // print all devices
    //                 println!("\nDevices Found: {}", all_device_managers.len());
    //                 for dm in all_device_managers.iter() {
    //                     println!(
    //                         "{} - {:?} - {}",
    //                         dm.system_id, dm.device_info.device_type, dm.device_info.name
    //                     );
    //                 }
    //                 println!("");
    //             }
    //         }
    //     }
    //     // Commands::Device { index, device_command} => {

    //     //     let device_manager = all_device_managers.get(index as usize).unwrap();

    //     //     println!("Device:      {:?}", device_manager.device_info.name);
    //     //     println!("System ID:   {:?}", device_manager.system_id);
    //     //     println!("Device Type: {:?}", device_manager.device_info.device_type);
    //     //     println!("Local ID:    {:?}", device_manager.device_info.id);

    //     //     process_device_command(device_command).unwrap();
    //     // }
    // }
}

fn process_device_command(device_command: DeviceCommand) -> Result<(), Error> {
    match device_command {
        DeviceCommand::Configs { config_command } => process_config_command(config_command)?,
    }
    Ok(())
}

fn process_config_command(config_command: ConfigCommand) -> Result<(), Error> {
    match config_command {
        ConfigCommand::list => {
            println!("Listing all available configs...");
        }
        ConfigCommand::set => {
            println!("Setting config...");
        }
        ConfigCommand::get => {
            println!("Getting current config...");
        }
    }
    Ok(())
}

// let mut mic = MicrophoneDevice::new(1, "test".to_string());

// let (data_tx, data_rx) = flume::bounded::<FramePacket>(32);

// mic.initialize(&cpal_host).unwrap();

// mic.start(
//     MicrophoneConfig::new(
//         16000,
//         1,
//         4
//     ),
//     None,
//     data_tx,
// ).unwrap();

// data_rx.into_iter().for_each(|frame| {
//     println!("Received frame: {:?}", frame);
// });

// println!("found {} devices", all_device_managers.len());
// for dev_man in all_device_managers.iter_mut() {
//     println!("{}", dev_man.device.get_device_info().name);
// }

// for dev_man in all_device_managers.iter_mut() {
//     let r = dev_man.device.instantiate_device();

//     match r {
//         Ok(_) => {}
//         Err(e) => {
//             println!("{}", e);
//             continue;
//         }
//     }

// let all_configs = match dev_man.device.get_all_available_configs() {
//     Ok(configs) => configs,
//     Err(err) => {
//         eprintln!("Error getting available configs: {}", err);
//         continue;
//     }
// };

// let all_configs = dev_man.device.get_all_available_configs().unwrap();

// for conf in all_configs.iter() {
//     println!("{:?} - {:?}", dev_man.device.get_device_info().name, conf)
// }
// }

// benchmark_sender(
//     _all_devices[0].device_info.clone(),
//     1000,
//     1000 * 1000 * 3,
//     1024,
// ).unwrap();

// let context = zmq::Context::new();

// let mut sender = Sender::new(
//     Ipv4Addr::new(127, 0, 0, 1),
//     10000,
//     10,
// );

// let mut receiver = Receiver::new(
//     Ipv4Addr::new(127, 0, 0, 1),
//     10000,
//     10,
// );

// sender.initialize(&context).unwrap();
// receiver.initialize(&context).unwrap();

// sender.send(FramePacket::new(
//     time::SystemTime::now(),
//     _all_devices[0].device_info.clone(),
//     vec![10, 20, 30],
//     generate_random_image(2550, 1550, 3),
// ), zmq::DONTWAIT).unwrap();

// match receiver.receive(zmq::DONTWAIT) {
//     Ok(res) => {
//         match res {
//             Some(frame) => {
//                 println!("Received data...");
//             },
//             None => {
//                 println!("No message available...");
//             }
//         }
//     },
//     Err(e) => {
//         println!("Error receiving data: {:?}", e);
//     }
// }
