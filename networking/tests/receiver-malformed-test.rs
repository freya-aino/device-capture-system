use std::thread;
use std::time::{Duration, Instant, SystemTime};

use networking::Receiver;
use shared::{ConnectionStatus, DeviceInformation, DeviceStatus, DeviceType, FramePacketInformation};

fn valid_parts(payload: &[u8]) -> Vec<Vec<u8>> {
    let info = FramePacketInformation {
        device_info: DeviceInformation {
            id: "id".to_string(),
            name: "name".to_string(),
            device_type: DeviceType::Camera,
            device_status: DeviceStatus::Available,
        },
        rx_timestamp: None,
        tx_timestamp: Some(SystemTime::now()),
        frame_shape: vec![2, 2],
    };
    vec![
        info.serialize().unwrap(),
        0u64.to_le_bytes().to_vec(),
        payload.to_vec(),
    ]
}

#[test]
fn receiver_skips_malformed_frames_and_returns_valid_ones() {
    let context = zmq::Context::new();
    let endpoint = "inproc://receiver-malformed-test";

    let publisher = context.socket(zmq::PUB).unwrap();
    publisher.bind(endpoint).unwrap();

    let mut receiver = Receiver::new_endpoint(endpoint, 16, 1024);
    receiver.start(&context).unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut received = None;

    while received.is_none() && Instant::now() < deadline {
        publisher.send_multipart([b"garbage".as_slice()], 0).unwrap();
        publisher
            .send_multipart(valid_parts(&[1, 2, 3, 4]), 0)
            .unwrap();
        thread::sleep(Duration::from_millis(20));

        if let Some(packet) = receiver.receive(zmq::DONTWAIT).unwrap() {
            received = Some(packet);
        }
    }

    let packet = received.expect("a valid frame should arrive despite malformed ones");
    assert_eq!(packet.data.as_ref(), &[1, 2, 3, 4]);
    assert!(packet.frame_info.rx_timestamp.is_some());
    assert_eq!(receiver.connection_status(), &ConnectionStatus::Active);

    receiver.stop().unwrap();
}
