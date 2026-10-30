use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::{Duration, Instant};

use access_client::{AccessClient, FrameSummary};
use anyhow::{Error, Result};

fn env_or<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    let hub_ip: Ipv4Addr = env_or("HUB_IP", Ipv4Addr::new(127, 0, 0, 1));
    let hub_port: u16 = env_or("HUB_PORT", 5556);
    let queue_size: u32 = env_or("CLIENT_QUEUE_SIZE", 128);
    let data_chunk_size: u32 = env_or("CLIENT_DATA_CHUNK_SIZE", 1024);
    // 0 (default): print one line per frame. N > 0: print one summary line per device every N seconds
    // (rate, bandwidth, latency, dropped frames) instead of the per-frame lines.
    let summary_secs: u64 = env_or("CLIENT_SUMMARY_SECS", 0);
    // 0 (default): run until killed. N > 0: exit cleanly after N seconds, printing a final summary.
    let run_secs: u64 = env_or("CLIENT_RUN_SECS", 0);

    println!("access-client: connecting to tcp://{}:{}", hub_ip, hub_port);

    let mut client = AccessClient::new(hub_ip, hub_port, queue_size, data_chunk_size);
    client.connect()?;

    let started = Instant::now();
    let deadline = (run_secs > 0).then(|| started + Duration::from_secs(run_secs));
    let mut summary =
        (summary_secs > 0).then(|| FrameSummary::new(Duration::from_secs(summary_secs)));

    loop {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        let wait = match deadline {
            Some(d) => d
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(5)),
            None => Duration::from_secs(5),
        };
        match client.next_frame(wait)? {
            Some(packet) => match summary.as_mut() {
                Some(summary) => summary.record(&packet),
                None => println!(
                    "frame: device={} type={:?} shape={:?} format={} bytes={}",
                    packet.frame_info.device_info.name,
                    packet.frame_info.device_info.device_type,
                    packet.frame_info.frame_shape,
                    packet.frame_info.pixel_format.as_deref().unwrap_or("-"),
                    packet.data.len()
                ),
            },
            None if summary.is_none() => {
                println!("access-client: no frame received in the last 5s, still waiting...")
            }
            None => {}
        }
        if let Some(summary) = summary.as_mut()
            && summary.window_elapsed()
        {
            for line in summary.flush(client.dropped_frames()) {
                println!("{line}");
            }
        }
    }

    if let Some(summary) = summary.as_mut() {
        for line in summary.flush(client.dropped_frames()) {
            println!("{line}");
        }
        for (device, total) in summary.totals() {
            println!(
                "final: device={device} total_frames={total} run_s={:.1} mean_rate={:.1}/s dropped_total={}",
                started.elapsed().as_secs_f64(),
                *total as f64 / started.elapsed().as_secs_f64(),
                client.dropped_frames()
            );
        }
    }
    client.disconnect()?;
    Ok(())
}
