use std::time::SystemTime;

use anyhow::{Result, bail};
use recorder::{RecorderConfig, StemAllocator, session_dir};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = RecorderConfig::from_env()?;

    let mut stems = StemAllocator::default();
    println!(
        "recorder: {} source from {} into {}",
        config.source.name(),
        config.hub_endpoint(),
        session_dir(&config.dir, SystemTime::now()).display()
    );
    for device in &config.devices {
        println!("recorder:   {device} -> {}.*", stems.stem(device));
    }

    bail!("recording is not implemented yet (SPECS.md §8 item 24; this binary is a scaffold)")
}
