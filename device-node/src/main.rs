use anyhow::{Error, Result, anyhow};
use device_node::{
    DEFAULT_CAMERA_CONFIG, DEFAULT_MICROPHONE_CONFIG, DEFAULT_STATUS_ADDR, DeviceNode,
    bind_status_server, get_all_devices, print_all_devices, start_status_server,
};
use shared::Device;
use std::{
    env::consts::OS,
    net::Ipv4Addr,
    str::FromStr,
    sync::{Arc, Mutex},
};

fn print_usage() {
    println!("Usage:");
    println!("  device-node list");
    println!("  device-node run <camera|mic> <device_index> <hub_ip> <hub_port>");
    println!("Set DEVICE_NODE_STATUS_ADDR to change the status endpoint (default {DEFAULT_STATUS_ADDR}).");
}

fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
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
            let status_addr = std::env::var("DEVICE_NODE_STATUS_ADDR")
                .unwrap_or_else(|_| DEFAULT_STATUS_ADDR.to_string());

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
                    run_node(
                        cam,
                        DEFAULT_CAMERA_CONFIG.clone(),
                        hub_ip,
                        hub_port,
                        &context,
                        &status_addr,
                    )?;
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
                    run_node(
                        mic,
                        DEFAULT_MICROPHONE_CONFIG.clone(),
                        hub_ip,
                        hub_port,
                        &context,
                        &status_addr,
                    )?;
                }
                other => return Err(anyhow!("Unknown device kind: {}", other)),
            }
        }
        Some(_) => print_usage(),
    }

    Ok(())
}

fn run_node<T>(
    device: T,
    config: T::Config,
    hub_ip: Ipv4Addr,
    hub_port: u16,
    context: &zmq::Context,
    status_addr: &str,
) -> Result<(), Error>
where
    T: Device + Send + 'static,
    T::Config: Clone + Send + 'static,
{
    let server = bind_status_server(status_addr)?;
    let mut node = DeviceNode::new(device, hub_ip, hub_port, 128, 1024);
    node.start(config, context)?;

    let node = Arc::new(Mutex::new(node));
    let _status_thread = start_status_server(server, node);
    println!("Status endpoint: http://{}/status", status_addr);

    wait_forever();
    Ok(())
}

fn wait_forever() {
    println!("Publishing... press Ctrl+C to stop.");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
