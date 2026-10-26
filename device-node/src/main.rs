use anyhow::{Error, Result, anyhow};
use device_node::{
    DEFAULT_CAMERA_CONFIG, DEFAULT_MICROPHONE_CONFIG, DeviceNode, get_all_devices,
    print_all_devices,
};
use shared::Device;
use std::{env::consts::OS, net::Ipv4Addr, str::FromStr};

fn print_usage() {
    println!("Usage:");
    println!("  device-node list");
    println!("  device-node run <camera|mic> <device_index> <hub_ip> <hub_port>");
}

fn main() -> Result<(), Error> {
    assert_eq!(OS, "linux", "Unsupported OS: {}", OS);

    let (all_cams, all_mics) = get_all_devices()?;
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        None | Some("list") => {
            print_all_devices(&all_cams, &all_mics);
        }
        Some("run") => {
            let kind = args.get(2).ok_or_else(|| anyhow!("Missing device kind"))?;
            let index: usize = args
                .get(3)
                .ok_or_else(|| anyhow!("Missing device index"))?
                .parse()?;
            let hub_ip = Ipv4Addr::from_str(args.get(4).ok_or_else(|| anyhow!("Missing hub ip"))?)?;
            let hub_port: u16 = args
                .get(5)
                .ok_or_else(|| anyhow!("Missing hub port"))?
                .parse()?;

            let context = zmq::Context::new();

            match kind.as_str() {
                "camera" => {
                    let cam = all_cams
                        .into_iter()
                        .nth(index)
                        .ok_or_else(|| anyhow!("No camera at index {}", index))?;
                    println!(
                        "Starting camera '{}' -> tcp://{}:{}",
                        cam.name(),
                        hub_ip,
                        hub_port
                    );
                    let mut node = DeviceNode::new(cam, hub_ip, hub_port, 128, 1024);
                    node.start(DEFAULT_CAMERA_CONFIG.clone(), &context)?;
                    wait_forever();
                }
                "mic" => {
                    let mic = all_mics
                        .into_iter()
                        .nth(index)
                        .ok_or_else(|| anyhow!("No microphone at index {}", index))?;
                    println!(
                        "Starting microphone '{}' -> tcp://{}:{}",
                        mic.name(),
                        hub_ip,
                        hub_port
                    );
                    let mut node = DeviceNode::new(mic, hub_ip, hub_port, 128, 1024);
                    node.start(DEFAULT_MICROPHONE_CONFIG.clone(), &context)?;
                    wait_forever();
                }
                other => return Err(anyhow!("Unknown device kind: {}", other)),
            }
        }
        Some(_) => print_usage(),
    }

    Ok(())
}

fn wait_forever() {
    println!("Publishing... press Ctrl+C to stop.");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
