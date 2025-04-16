use anyhow::{Error, Result};
use std::net::Ipv4Addr;
use std::time::{self, Duration};
use clap::{Parser, Subcommand};

use device_capture_system::modules::device::{DeviceManager, DeviceInformation};
use device_capture_system::{DeviceType, FramePacket, Receiver, Sender};




#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List,
    Camera {
        // device command
        #[clap(short, long)]
        index: u8,

        #[command(subcommand)]
        device_command: DeviceCommand,
    },
    Microphone {
        // device command
        #[clap(short, long)]
        index: u8,

        #[command(subcommand)]
        device_command: DeviceCommand,
    },
}

#[derive(Subcommand)]
pub enum DeviceCommand {
    ListConfigs,
    Info,
    Start,
    Stop,
    Pause,
    Resume,
    Terminate,
}


fn benchmark_sender(device_info: DeviceInformation, iterations: u32, data_size: u32, data_chunk_size: u32) -> Result<(), Error> {
    
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

        let fp = FramePacket::new(
            time::SystemTime::now(),
            device_info.clone(),
            vec![10, 20, 30],
            frame,
        );
        sender.send(fp, zmq::DONTWAIT, data_chunk_size).unwrap();

        let end_time = time::SystemTime::now();

        let duration = end_time.duration_since(start_time).unwrap();
        average_duration += duration;
    }

    println!("Benchmark completed!");
    println!("Average duration: {:?} -- Average fps: {:?}", average_duration.as_secs_f32() / (iterations as f32), 1.0 / (average_duration.as_secs_f32() / (iterations as f32)));

    Ok(())
}


fn main() {

    // init cli
    let _cli = Cli::parse();

    // get all devices
    let _all_devices = DeviceManager::get_all_available_devices_managed().unwrap();


    
    benchmark_sender(
        _all_devices[0].device_info.clone(), 
        1000,
        1000 * 1000 * 3,
        1024,
    ).unwrap();


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


    // match cli.command {
    //     Commands::List => {
    //         println!("\nDevices Found: {}", all_devices.len());
    //         for dev in all_devices.iter() {
    //             let type_str = match dev.device_info.device_type {
    //                 DeviceType::Camera => "Camera    ",
    //                 DeviceType::Microphone => "Microphone",
    //             };
    //             println!("{} : {} \t: {}", dev.system_id, type_str, dev.device_info.name);
    //         }
    //     }
    //     Commands::Camera { index, device_command } => {
    //         match device_command {
    //             DeviceCommand::Start => {
    //             }
    //             _ => {
    //                 println!("Camera command not implemented yet.");
    //             }
    //         }
    //     },
    //     Commands::Microphone { index, device_command } => {
    //         match device_command {
    //             DeviceCommand::Start => {
    //             }
    //             _ => {
    //                 println!("Microphone command not implemented yet.");
    //             }
    //         }
    //     }
    // }
}