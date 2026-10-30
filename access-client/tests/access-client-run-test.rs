use std::thread::sleep;
use std::time::Duration;

use access_client::AccessClient;
use networking::Sender;
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn frame_packet(payload: u8) -> FramePacket {
    let frame_info = FramePacketInformation {
        device_info: DeviceInformation {
            id: "run-test-device".to_string(),
            name: "run-test-device".to_string(),
            device_type: DeviceType::Microphone,
            device_status: DeviceStatus::Available,
        },
        rx_timestamp: None,
        tx_timestamp: None,
        frame_shape: vec![1],
        pixel_format: None,
        sample_rate: None,
    };
    FramePacket::new(frame_info, vec![payload].into_boxed_slice())
}

#[test]
fn run_delivers_frames_until_callback_returns_false() {
    let context = zmq::Context::new();
    let endpoint = "tcp://127.0.0.1:16301".to_string();

    let mut sender = Sender::new_endpoint(endpoint.clone(), 10, 1024);
    sender.start(&context).unwrap();

    let mut client = AccessClient::new_endpoint(endpoint, 10, 1024);
    client.connect().unwrap();
    sleep(Duration::from_millis(500));

    for payload in 1u8..=3 {
        sender.send(frame_packet(payload), zmq::DONTWAIT).unwrap();
    }

    let mut received = Vec::new();
    client
        .run(|packet| {
            received.push(packet.data[0]);
            received.len() < 3
        })
        .unwrap();

    assert_eq!(received, vec![1, 2, 3]);

    client.disconnect().unwrap();
    sender.stop().unwrap();
}
