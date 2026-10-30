use std::thread::sleep;
use std::time::{Duration, Instant};

use networking::{AlignedRelay, GroupReceiver, Sender};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn frame_packet(device_id: &str) -> FramePacket {
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
        pixel_format: None,
        sample_rate: None,
    };
    FramePacket::new(frame_info, vec![9u8, 8, 7, 6].into_boxed_slice())
}

#[test]
fn relay_publishes_one_group_with_a_frame_from_each_producer() {
    let context = zmq::Context::new();
    let producer_a_endpoint = "tcp://127.0.0.1:16501".to_string();
    let producer_b_endpoint = "tcp://127.0.0.1:16502".to_string();
    let output_endpoint = "tcp://127.0.0.1:16503".to_string();

    let mut producer_a = Sender::new_endpoint(producer_a_endpoint.clone(), 10, 1024);
    producer_a.start(&context).unwrap();
    let mut producer_b = Sender::new_endpoint(producer_b_endpoint.clone(), 10, 1024);
    producer_b.start(&context).unwrap();

    let mut relay = AlignedRelay::new(
        vec![producer_a_endpoint, producer_b_endpoint],
        output_endpoint.clone(),
        vec!["producer-a".to_string(), "producer-b".to_string()],
        Duration::from_millis(500),
    );
    relay.start(&context).unwrap();

    let mut receiver = GroupReceiver::new_endpoint(output_endpoint, 10, 1024);
    receiver.start(&context).unwrap();

    let mut group = None;
    let start = Instant::now();
    while group.is_none() && start.elapsed() < Duration::from_secs(5) {
        producer_a
            .send(frame_packet("producer-a"), zmq::DONTWAIT)
            .unwrap();
        producer_b
            .send(frame_packet("producer-b"), zmq::DONTWAIT)
            .unwrap();
        sleep(Duration::from_millis(100));
        group = receiver.receive(zmq::DONTWAIT).unwrap();
    }

    let group = group.expect("relay should publish an aligned group");
    let mut ids: Vec<&str> = group
        .frames
        .iter()
        .map(|f| f.frame_info.device_info.id.as_str())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["producer-a", "producer-b"]);
    assert_eq!(group.frames[0].data.as_ref(), &[9u8, 8, 7, 6]);
    assert!(group.skew <= Duration::from_millis(500));

    receiver.stop().unwrap();
    relay.stop().unwrap();
    producer_a.stop().unwrap();
    producer_b.stop().unwrap();
}

#[test]
fn relay_publishes_nothing_while_one_producer_is_silent() {
    let context = zmq::Context::new();
    let producer_a_endpoint = "tcp://127.0.0.1:16511".to_string();
    let producer_b_endpoint = "tcp://127.0.0.1:16512".to_string();
    let output_endpoint = "tcp://127.0.0.1:16513".to_string();

    let mut producer_a = Sender::new_endpoint(producer_a_endpoint.clone(), 10, 1024);
    producer_a.start(&context).unwrap();
    let mut producer_b = Sender::new_endpoint(producer_b_endpoint.clone(), 10, 1024);
    producer_b.start(&context).unwrap();

    let mut relay = AlignedRelay::new(
        vec![producer_a_endpoint, producer_b_endpoint],
        output_endpoint.clone(),
        vec!["producer-a".to_string(), "producer-b".to_string()],
        Duration::from_millis(500),
    );
    relay.start(&context).unwrap();

    let mut receiver = GroupReceiver::new_endpoint(output_endpoint, 10, 1024);
    receiver.start(&context).unwrap();
    sleep(Duration::from_millis(500));

    producer_a
        .send(frame_packet("producer-a"), zmq::DONTWAIT)
        .unwrap();
    sleep(Duration::from_millis(300));

    assert!(receiver.receive(zmq::DONTWAIT).unwrap().is_none());

    receiver.stop().unwrap();
    relay.stop().unwrap();
    producer_a.stop().unwrap();
    producer_b.stop().unwrap();
}
