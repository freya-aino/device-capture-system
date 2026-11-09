//! The forwarding loop: newest frame of the selected device from the hub into the sink.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use access_client::AccessClient;
use anyhow::{Result, ensure};
use shared::FramePacket;

use crate::convert::convert_frame;
use crate::sink::{FrameSink, V4l2Output};
use crate::{LiveConfig, OutputPlan, plan_output};

/// Receive queue and part size for the hub connection.
const QUEUE_SIZE: u32 = 64;
const CHUNK_SIZE: u32 = 1024;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ForwardStats {
    /// Frames written to the sink.
    pub written: u64,
    /// Frames of the selected device that arrived while an older one was being written, and were
    /// skipped so the sink shows the newest.
    pub skipped: u64,
    /// The output decided from the first frame, when one arrived.
    pub plan: Option<OutputPlan>,
}

/// Forwards into the configured `v4l2loopback` device until `stop` is set.
pub fn forward(config: &LiveConfig, stop: &AtomicBool) -> Result<ForwardStats> {
    let sink = config.sink.clone();
    forward_with(config, stop, move |plan| {
        let output = V4l2Output::open(&sink, plan.clone())?;
        tracing::info!(
            "{}: {} {}x{} -> {} {}",
            sink.display(),
            plan.input_format,
            plan.width,
            plan.height,
            plan.output.fourcc(),
            match plan.conversion {
                crate::Conversion::Passthrough => "unchanged",
                _ => "converted",
            }
        );
        Ok(Box::new(output) as Box<dyn FrameSink>)
    })
}

/// `forward` with the sink supplied by `open`, called once with the plan of the first frame.
pub fn forward_with(
    config: &LiveConfig,
    stop: &AtomicBool,
    mut open: impl FnMut(&OutputPlan) -> Result<Box<dyn FrameSink>>,
) -> Result<ForwardStats> {
    let mut client = AccessClient::new_endpoint(config.hub_endpoint(), QUEUE_SIZE, CHUNK_SIZE);
    client.connect()?;
    let mut stats = ForwardStats::default();
    let mut sink: Option<Box<dyn FrameSink>> = None;
    let mut scratch = Vec::new();

    let result = (|| -> Result<()> {
        while !stop.load(Ordering::SeqCst) {
            let Some(packet) = newest_frame(&mut client, &config.device, &mut stats.skipped)?
            else {
                continue;
            };
            let info = &packet.frame_info;
            let format = info.pixel_format.as_deref().unwrap_or("");
            let plan = match &stats.plan {
                Some(plan) => {
                    ensure!(
                        plan.input_format == format
                            && plan.width == info.frame_shape.first().copied().unwrap_or(0)
                            && plan.height == info.frame_shape.get(1).copied().unwrap_or(0),
                        "stream changed from {} {}x{} to {} {:?}; the sink's format is fixed",
                        plan.input_format,
                        plan.width,
                        plan.height,
                        format,
                        info.frame_shape
                    );
                    plan
                }
                None => {
                    let plan = plan_output(format, &info.frame_shape)?;
                    sink = Some(open(&plan)?);
                    stats.plan.insert(plan)
                }
            };
            let bytes = convert_frame(plan, &packet.data, &mut scratch)?;
            sink.as_mut()
                .expect("sink opens with the plan")
                .write_frame(bytes)?;
            stats.written += 1;
        }
        Ok(())
    })();
    if let Err(e) = client.disconnect() {
        tracing::warn!("disconnecting from the hub: {e:#}");
    }
    result.map(|()| stats)
}

/// Waits up to 100 ms for a frame, then drains everything already queued and returns the newest
/// frame of `device`, counting the older ones it skipped. `None` when nothing for `device` arrived.
fn newest_frame(
    client: &mut AccessClient,
    device: &str,
    skipped: &mut u64,
) -> Result<Option<FramePacket>> {
    let mut newest: Option<FramePacket> = None;
    let mut timeout = Duration::from_millis(100);
    while let Some(packet) = client.next_frame(timeout)? {
        timeout = Duration::ZERO;
        if packet.frame_info.device_info.id == device {
            if newest.is_some() {
                *skipped += 1;
            }
            newest = Some(packet);
        }
    }
    Ok(newest)
}
