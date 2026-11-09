use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use recorder::{RecorderConfig, record};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let config = RecorderConfig::from_env()?;

    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    ctrlc::set_handler(move || {
        eprintln!("recorder: stopping");
        flag.store(true, Ordering::SeqCst);
    })?;

    match config.duration {
        Some(d) => println!(
            "recorder: {} source from {} for {} s (Ctrl+C stops early)",
            config.source.name(),
            config.hub_endpoint(),
            d.as_secs()
        ),
        None => println!(
            "recorder: {} source from {} until Ctrl+C",
            config.source.name(),
            config.hub_endpoint()
        ),
    }

    let recording = record(&config, &stop)?;
    println!("recorder: session {}", recording.dir.display());
    for device in &recording.info.devices {
        println!(
            "recorder:   {} -> {}.* ({}): {} written, {} dropped, {} gaps{}",
            device.device_id,
            device.file_stem,
            device.codec,
            device.frames_written,
            device.frames_dropped,
            device.sequence_gaps,
            device
                .error
                .as_ref()
                .map(|e| format!(", stopped early: {e}"))
                .unwrap_or_default()
        );
    }
    Ok(())
}
