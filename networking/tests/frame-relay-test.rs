use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};

use networking::{FrameRelay, Receiver, Sender};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn frame_packet(device_id: &str, data: Vec<u8>) -> FramePacket {
    let frame_info = FramePacketInformation {
        device_info: DeviceInformation {
            id: device_id.to_string(),
            name: "test-device".to_string(),
            device_type: DeviceType::Camera,
            device_status: DeviceStatus::Available,
        },
        rx_timestamp: None,
        tx_timestamp: None,
        frame_shape: vec![2, 2],
        pixel_format: Some("GREY".to_string()),
        sample_rate: None,
        capture_timestamp: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(7)),
    };
    FramePacket::new(frame_info, data.into_boxed_slice())
}

#[test]
fn relay_publishes_transformed_frames_and_skips_the_rest() {
    let context = zmq::Context::new();
    let producer_endpoint = "tcp://127.0.0.1:16521".to_string();
    let output_endpoint = "tcp://127.0.0.1:16522".to_string();

    let mut producer = Sender::new_endpoint(producer_endpoint.clone(), 10, 1024);
    producer.start(&context).unwrap();

    // Invert the cam's pixels, change its shape, skip every other device, fail on "bad".
    let mut relay = FrameRelay::new(
        vec![producer_endpoint],
        output_endpoint.clone(),
        3,
        Box::new(
            |mut packet: FramePacket| match packet.frame_info.device_info.id.as_str() {
                "cam" => {
                    packet.data = packet.data.iter().map(|v| 255 - v).collect();
                    packet.frame_info.frame_shape = vec![4, 1];
                    Ok(Some(packet))
                }
                "bad" => Err(anyhow::anyhow!("transform failed")),
                _ => Ok(None),
            },
        ),
    );
    relay.start(&context).unwrap();

    let mut receiver = Receiver::new_endpoint(output_endpoint, 10, 3);
    receiver.start(&context).unwrap();

    let mut received = None;
    let start = Instant::now();
    while received.is_none() && start.elapsed() < Duration::from_secs(5) {
        for device in ["other", "bad", "cam"] {
            producer
                .send(frame_packet(device, vec![0, 10, 20, 30]), zmq::DONTWAIT)
                .unwrap();
        }
        sleep(Duration::from_millis(100));
        while let Some(frame) = receiver.receive(zmq::DONTWAIT).unwrap() {
            assert_eq!(
                frame.frame_info.device_info.id, "cam",
                "only cam frames are published"
            );
            received.get_or_insert(frame);
        }
    }

    let frame = received.expect("relay should publish the cam's processed frames");
    assert_eq!(frame.data.as_ref(), &[255u8, 245, 235, 225]);
    assert_eq!(frame.frame_info.frame_shape, vec![4, 1]);
    assert_eq!(
        frame.frame_info.capture_timestamp,
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(7))
    );

    receiver.stop().unwrap();
    relay.stop().unwrap();
    producer.stop().unwrap();
}
