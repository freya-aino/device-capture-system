use anyhow::{Error, Result, anyhow, bail};
use device_node::{
    CameraSelection, DEFAULT_MICROPHONE_CONFIG, DEFAULT_STATUS_ADDR, DeviceNode, NodeConfig,
    NodeDeviceType, bind_status_server, get_all_devices, print_all_devices, resolve_camera_mode,
    start_status_server, take_device_by_id,
};
use devices::{AlsaMicrophoneDevice, V4lCameraDevice};
use shared::{CameraConfig, Device, MicrophoneConfig};
use std::{
    env::consts::OS,
    net::{Ipv4Addr, SocketAddrV4},
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};

fn print_usage() {
    println!("Usage:");
    println!("  device-node list");
    println!("  device-node run <camera|mic> <device_index> <bind_ip> <bind_port>");
    println!("  device-node run --config <node.toml>");
    println!(
        "  device-node config save <node.toml> run <camera|mic> <device_index> <bind_ip> <bind_port>"
    );
    println!(
        "Set DEVICE_NODE_STATUS_ADDR to change the status endpoint (default {DEFAULT_STATUS_ADDR})."
    );
    println!("Camera config selection (from the modes the camera advertises):");
    println!(
        "  DEVICE_NODE_CAMERA_POLICY=nearest|best-fps|best-resolution|best-average (default nearest)"
    );
    println!(
        "  DEVICE_NODE_CAMERA_WIDTH / _HEIGHT / _FPS   targets for `nearest` (default 1920 1080 30)"
    );
    println!("`config save` resolves the arguments as `run` would and writes them by device id;");
    println!("`run --config` starts from the file and ignores the DEVICE_NODE_* variables.");
}

/// A device and the mode it will stream, resolved from CLI arguments or a saved config.
enum Resolved {
    Camera(V4lCameraDevice, CameraConfig),
    Microphone(AlsaMicrophoneDevice, MicrophoneConfig),
}

impl Resolved {
    fn device_id(&self) -> &str {
        match self {
            Resolved::Camera(cam, _) => cam.id(),
            Resolved::Microphone(mic, _) => mic.id(),
        }
    }

    fn describe(&self) -> String {
        match self {
            Resolved::Camera(cam, config) => format!("camera '{}' with {:?}", cam.name(), config),
            Resolved::Microphone(mic, config) => {
                format!("microphone '{}' with {:?}", mic.name(), config)
            }
        }
    }
}

struct Plan {
    resolved: Resolved,
    bind: SocketAddrV4,
    status_addr: String,
}

fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    assert_eq!(OS, "linux", "Unsupported OS: {}", OS);

    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        None | Some("list") => {
            let (all_cams, all_mics) = get_all_devices()?;
            print_all_devices(&all_cams, &all_mics);
        }
        Some("run") if args.get(2).map(String::as_str) == Some("--config") => {
            let path = args
                .get(3)
                .ok_or_else(|| anyhow!("Missing config path after --config"))?;
            let plan = plan_from_config(Path::new(path))?;
            println!("Config {path}: {}", plan.resolved.describe());
            run(plan)?;
        }
        Some("run") => {
            let plan = plan_from_args(&args[2..])?;
            println!("Starting {}", plan.resolved.describe());
            run(plan)?;
        }
        Some("config") => {
            if args.get(2).map(String::as_str) != Some("save") {
                print_usage();
                bail!("Unknown config command; expected `config save <path> run ...`");
            }
            let path = args
                .get(3)
                .ok_or_else(|| anyhow!("Missing config path after `config save`"))?;
            if args.get(4).map(String::as_str) != Some("run") {
                print_usage();
                bail!("`config save <path>` must be followed by `run <camera|mic> ...`");
            }
            let plan = plan_from_args(&args[5..])?;
            let config = match &plan.resolved {
                Resolved::Camera(cam, mode) => {
                    NodeConfig::camera(cam.id(), mode, plan.bind, &plan.status_addr)
                }
                Resolved::Microphone(mic, mode) => {
                    NodeConfig::microphone(mic.id(), mode, plan.bind, &plan.status_addr)
                }
            };
            config.save(path)?;
            println!(
                "Saved {} for {} to {path}:\n{}",
                plan.resolved.describe(),
                plan.resolved.device_id(),
                config.to_toml()
            );
        }
        Some(_) => print_usage(),
    }

    Ok(())
}

/// `run <camera|mic> <index> <ip> <port>` resolved as the node has always done it: the device by
/// index, the camera mode by `DEVICE_NODE_CAMERA_*`, the status address by its variable.
fn plan_from_args(args: &[String]) -> Result<Plan> {
    let kind = args.first().ok_or_else(|| anyhow!("Missing device kind"))?;
    let index: usize = args
        .get(1)
        .ok_or_else(|| anyhow!("Missing device index"))?
        .parse()?;
    let ip = Ipv4Addr::from_str(args.get(2).ok_or_else(|| anyhow!("Missing bind ip"))?)?;
    let port: u16 = args
        .get(3)
        .ok_or_else(|| anyhow!("Missing bind port"))?
        .parse()?;
    let status_addr = std::env::var("DEVICE_NODE_STATUS_ADDR")
        .unwrap_or_else(|_| DEFAULT_STATUS_ADDR.to_string());
    let (all_cams, all_mics) = get_all_devices()?;

    let resolved = match kind.as_str() {
        "camera" => {
            let cam = all_cams
                .into_iter()
                .nth(index)
                .ok_or_else(|| anyhow!("No camera at index {}", index))?;
            let selection = CameraSelection::from_env()?;
            let configs = cam.get_configs()?;
            let config = selection
                .choose(&configs)
                .ok_or_else(|| anyhow!("Camera '{}' advertises no capture configs", cam.name()))?;
            println!(
                "Selection {:?} picked {:?} out of {} advertised configs",
                selection,
                config,
                configs.len()
            );
            Resolved::Camera(cam, config)
        }
        "mic" => {
            let mic = all_mics
                .into_iter()
                .nth(index)
                .ok_or_else(|| anyhow!("No microphone at index {}", index))?;
            Resolved::Microphone(mic, DEFAULT_MICROPHONE_CONFIG.clone())
        }
        other => return Err(anyhow!("Unknown device kind: {}", other)),
    };
    Ok(Plan {
        resolved,
        bind: SocketAddrV4::new(ip, port),
        status_addr,
    })
}

/// A saved config: the device by id, the exact saved mode. Missing device or mode is an error;
/// nothing is substituted and environment variables are not consulted.
fn plan_from_config(path: &Path) -> Result<Plan> {
    let config = NodeConfig::load(path)?;
    let (all_cams, all_mics) = get_all_devices()?;
    let resolved = match config.device_type {
        NodeDeviceType::Camera => {
            let cam = take_device_by_id(all_cams, &config.device_id)?;
            let mode = resolve_camera_mode(&cam, config.camera_section()?)?;
            Resolved::Camera(cam, mode)
        }
        NodeDeviceType::Microphone => {
            let mic = take_device_by_id(all_mics, &config.device_id)?;
            Resolved::Microphone(mic, config.microphone_section()?.to_config())
        }
    };
    Ok(Plan {
        resolved,
        bind: config.bind_addr()?,
        status_addr: config.status_addr.clone(),
    })
}

fn run(plan: Plan) -> Result<()> {
    let context = zmq::Context::new();
    println!("Publishing on tcp://{}", plan.bind);
    match plan.resolved {
        Resolved::Camera(cam, config) => {
            run_node(cam, config, plan.bind, &context, &plan.status_addr)
        }
        Resolved::Microphone(mic, config) => {
            run_node(mic, config, plan.bind, &context, &plan.status_addr)
        }
    }
}

fn run_node<T>(
    device: T,
    config: T::Config,
    bind: SocketAddrV4,
    context: &zmq::Context,
    status_addr: &str,
) -> Result<(), Error>
where
    T: Device + Send + 'static,
    T::Config: Clone + Send + 'static,
{
    let server = bind_status_server(status_addr)?;
    let mut node = DeviceNode::new(device, *bind.ip(), bind.port(), 128, 1024);
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
