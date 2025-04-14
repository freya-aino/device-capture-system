use std::net::Ipv4Addr;
use clap::{Parser, Subcommand};

use device_capture_system::modules::network::Connection;
use device_capture_system::modules::device::DeviceManager;




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

fn main() {

    let _cli = Cli::parse();

    let _all_devices = DeviceManager::get_all_available_devices_managed().unwrap();

    let mut conn = Connection::new(
        Ipv4Addr::new(127, 0, 0, 1),
        10000,
        10,
    );

    let context = zmq::Context::new();
    // conn.initialize_as_sender(&context).unwrap();
    
    


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