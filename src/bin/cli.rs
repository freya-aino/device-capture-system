use std::net::Ipv4Addr;
use std::time::{self, Duration};
use clap::{Parser, Subcommand};

use device_capture_system::modules::device::DeviceManager;
use device_capture_system::{FramePacket, Sender};




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


fn generate_random_image(w: u32, h: u32, c: u32) -> Vec<u8> {
    let size = (w * h * c) as usize;
    let arr = vec![0u8; size];
    arr
}


fn main() {

    let _cli = Cli::parse();

    let _all_devices = DeviceManager::get_all_available_devices_managed().unwrap();

    let mut sender = Sender::new(
        Ipv4Addr::new(127, 0, 0, 1),
        10000,
        10,
    );

    let context = zmq::Context::new();
    sender.initialize(&context).unwrap();
    
    let fp = FramePacket::new(
        time::SystemTime::now(),
        _all_devices[0].device_info.clone(),
        vec![10, 20, 30],
        generate_random_image(640, 480, 3).as_slice(),
    );

    let frame = generate_random_image(2550, 1550, 3);

    let mut average_duration: Duration = Duration::new(0, 0);
    // time sending frames
    let num_tests = 10.0;
    for i in 0..(num_tests as i32) {

        let start_time = time::SystemTime::now();

        let fp = FramePacket::new(
            time::SystemTime::now(),
            _all_devices[0].device_info.clone(),
            vec![10, 20, 30],
            &frame,
        );
        sender.send(fp).unwrap();

        let end_time = time::SystemTime::now();

        let duration = end_time.duration_since(start_time).unwrap();
        average_duration += duration;
    }

    println!("Average duration: {:?} -- Average fps: {:?}", average_duration.as_secs_f32() / num_tests, 1.0 / (average_duration.as_secs_f32() / num_tests));


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