//! A loopback hub for the pytest: a proxy on the usual ports, an aligned relay on the sync port,
//! and two synthetic YUYV cameras whose frames share a capture time.

use std::thread::sleep;
use std::time::{Duration, SystemTime};

use networking::{AlignedRelay, Proxy, Sender};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn frame(id: &str, captured: SystemTime) -> FramePacket {
    FramePacket::new(
        FramePacketInformation {
            device_info: DeviceInformation {
                id: id.to_string(),
                name: id.to_string(),
                device_type: DeviceType::Camera,
                device_status: DeviceStatus::Running,
            },
            rx_timestamp: None,
            tx_timestamp: None,
            frame_shape: vec![2, 1],
            pixel_format: Some("YUYV".to_string()),
            sample_rate: None,
            capture_timestamp: Some(captured),
        },
        // Two YUYV pixels: black (Y 16) and white (Y 235), neutral chroma.
        vec![16, 128, 235, 128].into_boxed_slice(),
    )
}

fn main() -> anyhow::Result<()> {
    let context = zmq::Context::new();

    let mut hub = Proxy::new_endpoints(
        "tcp://127.0.0.1:5555".to_string(),
        "tcp://127.0.0.1:5556".to_string(),
        10,
        1024,
    );
    hub.add_from_endpoint("tcp://127.0.0.1:5554".to_string());
    hub.start(&context)?;
    let mut relay = AlignedRelay::new(
        vec!["tcp://127.0.0.1:5556".to_string()],
        "tcp://127.0.0.1:5557",
        vec![
            "loopback-camera".to_string(),
            "loopback-camera-2".to_string(),
        ],
        Duration::from_millis(20),
    );
    relay.start(&context)?;

    let mut producer_a = Sender::new_endpoint("tcp://127.0.0.1:5555".to_string(), 10, 1024);
    producer_a.start(&context)?;
    let mut producer_b = Sender::new_endpoint("tcp://127.0.0.1:5554".to_string(), 10, 1024);
    producer_b.start(&context)?;

    sleep(Duration::from_millis(500));

    println!("publishing on tcp://127.0.0.1:5556 (raw) and tcp://127.0.0.1:5557 (aligned)");
    for _ in 0..400u32 {
        let captured = SystemTime::now();
        producer_a.send(frame("loopback-camera", captured), zmq::DONTWAIT)?;
        producer_b.send(frame("loopback-camera-2", captured), zmq::DONTWAIT)?;
        sleep(Duration::from_millis(50));
    }

    producer_a.stop()?;
    producer_b.stop()?;
    relay.stop()?;
    hub.stop()?;
    Ok(())
}
