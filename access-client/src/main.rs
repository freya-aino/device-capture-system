use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;

use access_client::AccessClient;
use anyhow::{Error, Result};

fn env_or<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

fn main() -> Result<(), Error> {
    let hub_ip: Ipv4Addr = env_or("HUB_IP", Ipv4Addr::new(127, 0, 0, 1));
    let hub_port: u16 = env_or("HUB_PORT", 5556);
    let queue_size: u32 = env_or("CLIENT_QUEUE_SIZE", 128);
    let data_chunk_size: u32 = env_or("CLIENT_DATA_CHUNK_SIZE", 1024);

    println!("access-client: connecting to tcp://{}:{}", hub_ip, hub_port);

    let mut client = AccessClient::new(hub_ip, hub_port, queue_size, data_chunk_size);
    client.connect()?;

    loop {
        match client.next_frame(Duration::from_secs(5))? {
            Some(packet) => {
                println!(
                    "frame: device={} type={:?} shape={:?} bytes={}",
                    packet.frame_info.device_info.name,
                    packet.frame_info.device_info.device_type,
                    packet.frame_info.frame_shape,
                    packet.data.len()
                );
            }
            None => println!("access-client: no frame received in the last 5s, still waiting..."),
        }
    }
}
