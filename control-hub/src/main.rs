use std::net::{Ipv4Addr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Error, Result, anyhow};
use networking::Proxy;

mod status;
use status::DEFAULT_HUB_STATUS_ADDR;

struct HubConfig {
    producers: Vec<SocketAddrV4>,
    consumer_ip: Ipv4Addr,
    consumer_port: u16,
    queue_size: u32,
    data_chunk_size: u32,
    status_addr: String,
}

fn env_or<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

/// Parses `HUB_PRODUCERS`, a comma-separated list of `ip:port` producer endpoints.
fn parse_producers(raw: &str) -> Result<Vec<SocketAddrV4>> {
    let producers = raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            SocketAddrV4::from_str(entry)
                .map_err(|e| anyhow!("invalid HUB_PRODUCERS entry {:?}: {}", entry, e))
        })
        .collect::<Result<Vec<_>>>()?;

    if producers.is_empty() {
        return Err(anyhow!("HUB_PRODUCERS is set but contains no endpoints"));
    }
    Ok(producers)
}

impl HubConfig {
    /// Loads bind addresses/ports and transport tuning from the environment, falling
    /// back to sane defaults so `control-hub` runs out of the box on one machine.
    /// `HUB_PRODUCERS` takes precedence over the single-endpoint `HUB_PRODUCER_IP`/`HUB_PRODUCER_PORT`.
    fn from_env() -> Result<Self> {
        let producers = match std::env::var("HUB_PRODUCERS") {
            Ok(raw) => parse_producers(&raw)?,
            // `networking::Proxy` *connects* its XSUB side to producer endpoints,
            // so the default must be a concrete address, not 0.0.0.0.
            Err(_) => vec![SocketAddrV4::new(
                env_or("HUB_PRODUCER_IP", Ipv4Addr::new(127, 0, 0, 1)),
                env_or("HUB_PRODUCER_PORT", 5555),
            )],
        };

        Ok(HubConfig {
            producers,
            consumer_ip: env_or("HUB_CONSUMER_IP", Ipv4Addr::new(0, 0, 0, 0)),
            consumer_port: env_or("HUB_CONSUMER_PORT", 5556),
            queue_size: env_or("HUB_QUEUE_SIZE", 128),
            data_chunk_size: env_or("HUB_DATA_CHUNK_SIZE", 1024),
            status_addr: env_or("HUB_STATUS_ADDR", DEFAULT_HUB_STATUS_ADDR.to_string()),
        })
    }
}

fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    let config = HubConfig::from_env()?;

    let (first, rest) = config
        .producers
        .split_first()
        .ok_or_else(|| anyhow!("no producer endpoints configured"))?;

    println!(
        "control-hub: {} producer endpoint(s) (XSUB connect): {}; consumers -> tcp://{}:{} (XPUB bind)",
        config.producers.len(),
        config
            .producers
            .iter()
            .map(|p| format!("tcp://{}", p))
            .collect::<Vec<_>>()
            .join(", "),
        config.consumer_ip,
        config.consumer_port
    );

    let context = zmq::Context::new();
    let mut proxy = Proxy::new(
        *first.ip(),
        first.port(),
        config.consumer_ip,
        config.consumer_port,
        config.queue_size,
        config.data_chunk_size,
    );
    for producer in rest {
        proxy.add_from_endpoint(format!("tcp://{}", producer));
    }

    proxy.start(&context)?;
    let status_server = status::bind_status_server(&config.status_addr)?;
    let _status_thread = status::start_status_server(status_server, proxy.peer_counts());
    println!(
        "control-hub: proxy running, status on http://{}/status, press Ctrl+C to stop",
        config.status_addr
    );

    let running = Arc::new(AtomicBool::new(true));
    let running_handler = running.clone();
    ctrlc::set_handler(move || running_handler.store(false, Ordering::SeqCst))
        .map_err(|e| Error::msg(format!("Failed to set Ctrl+C handler: {}", e)))?;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(200));
    }

    println!("control-hub: shutting down proxy");
    proxy.stop()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Environment variables are process-global, so tests touching them must not run
    // concurrently with each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn env_or_falls_back_to_default_when_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("TEST_ENV_OR_MISSING");
        }
        let value: u16 = env_or("TEST_ENV_OR_MISSING", 1234);
        assert_eq!(value, 1234);
    }

    #[test]
    fn env_or_parses_set_value() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("TEST_ENV_OR_SET", "4242");
        }
        let value: u16 = env_or("TEST_ENV_OR_SET", 1234);
        assert_eq!(value, 4242);
        unsafe {
            std::env::remove_var("TEST_ENV_OR_SET");
        }
    }

    #[test]
    fn env_or_falls_back_on_unparseable_value() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("TEST_ENV_OR_INVALID", "not-a-number");
        }
        let value: u16 = env_or("TEST_ENV_OR_INVALID", 1234);
        assert_eq!(value, 1234);
        unsafe {
            std::env::remove_var("TEST_ENV_OR_INVALID");
        }
    }

    #[test]
    fn parse_producers_accepts_comma_separated_endpoints() {
        let producers = parse_producers(" 10.0.0.2:5555, 10.0.0.3:5557 ,").unwrap();
        assert_eq!(
            producers,
            vec![
                SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 2), 5555),
                SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 3), 5557),
            ]
        );
    }

    #[test]
    fn parse_producers_rejects_invalid_entry() {
        assert!(parse_producers("10.0.0.2:5555,not-an-endpoint").is_err());
    }

    #[test]
    fn parse_producers_rejects_empty_list() {
        assert!(parse_producers(" , ").is_err());
    }
}
