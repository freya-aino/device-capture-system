use anyhow::{Result, bail};
use live_sink::LiveConfig;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = LiveConfig::from_env()?;
    println!(
        "live-sink: {} from {} -> {}",
        config.device,
        config.hub_endpoint(),
        config.sink.display()
    );
    bail!("the V4L2 writer is not implemented yet (SPECS.md §8 item 25; this binary is a scaffold)")
}
