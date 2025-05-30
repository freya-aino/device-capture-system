use clap::{Parser, Subcommand};
use devices::{CpalMicrophoneDevice, DeviceManager, V4lCameraDevice};
use shared::start_static_http_server;
use std::{env::consts::OS, process::Command};

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

fn main() {
    assert_eq!(OS, "linux", "Unsupported OS: {}", OS);

    // get all devices
    let cpal_host = cpal::default_host();
    let camera_dms = DeviceManager::<V4lCameraDevice>::get_all_devices().unwrap();
    let microphone_dms =
        DeviceManager::<CpalMicrophoneDevice>::get_all_devices(&cpal_host).unwrap();

    // cli commands
    match Cli::parse().command {
        ClapCommand::Init => {
            println!("Initializing device node...");
            println!("Creating cache directory...");
            let mut home_path = std::env::var("HOME").unwrap();
            home_path.push_str("/.cache/device-capture-device-node/");
            let out = Command::new("mkdir")
                .arg("-p")
                .arg(home_path)
                .output()
                .unwrap();
            println!(
                "{}{}",
                String::from_utf8_lossy(&out.stderr),
                String::from_utf8_lossy(&out.stdout)
            );

            println!("Saving Configuration")
        }
        ClapCommand::List { verbose } => {
            println!("--> {} camera devices found:", camera_dms.len());

            for dm in camera_dms {
                println!("{:?}", dm.device.device_info);

                if verbose {}
            }

            println!("--> {} microphone devices found:", microphone_dms.len());

            for dm in microphone_dms {
                println!("{:?}", dm.device.device_info);

                if verbose {}
            }
        }
        ClapCommand::Run { host, port } => {
            println!("Running device node on {}:{}...", host, port);
        }
    }

    // start_static_http_server("0.0.0.0", "8080", ())
}
