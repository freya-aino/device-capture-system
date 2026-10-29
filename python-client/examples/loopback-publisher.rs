use std::thread::sleep;
use std::time::Duration;

use networking::{Proxy, Sender};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn main() -> anyhow::Result<()> {
    let context = zmq::Context::new();

    let mut hub = Proxy::new_endpoints(
        "tcp://127.0.0.1:5555".to_string(),
        "tcp://127.0.0.1:5556".to_string(),
        10,
        1024,
    );
    hub.start(&context)?;

    let mut producer = Sender::new_endpoint("tcp://127.0.0.1:5555".to_string(), 10, 1024);
    producer.start(&context)?;

    sleep(Duration::from_millis(500));

    println!("publishing on tcp://127.0.0.1:5556 (consumer side of the hub)");
    for i in 0..200u32 {
        let frame_info = FramePacketInformation {
            device_info: DeviceInformation {
                id: "loopback-camera".to_string(),
                name: "loopback-camera".to_string(),
                device_type: DeviceType::Camera,
                device_status: DeviceStatus::Running,
            },
            rx_timestamp: None,
            tx_timestamp: None,
            frame_shape: vec![2, 2],
            pixel_format: Some("YUYV".to_string()),
        };
        let data = vec![i as u8, 1, 2, 3].into_boxed_slice();
        producer.send(FramePacket::new(frame_info, data), zmq::DONTWAIT)?;
        sleep(Duration::from_millis(50));
    }

    producer.stop()?;
    hub.stop()?;
    Ok(())
}
