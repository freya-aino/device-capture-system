use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use live_sink::{LiveConfig, forward};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let config = LiveConfig::from_env()?;
    println!(
        "live-sink: {} from {} -> {} (Ctrl+C stops)",
        config.device,
        config.hub_endpoint(),
        config.sink.display()
    );

    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    ctrlc::set_handler(move || {
        eprintln!("live-sink: stopping");
        flag.store(true, Ordering::SeqCst);
    })?;

    let stats = forward(&config, &stop)?;
    println!(
        "live-sink: {} frames written, {} skipped (newest-frame policy)",
        stats.written, stats.skipped
    );
    Ok(())
}
