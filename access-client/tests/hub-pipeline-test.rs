use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use access_client::AccessClient;
use networking::{Proxy, Sender};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn generate_dummy_frame_packet() -> FramePacket {
    generate_dummy_frame_packet_with_id("test-device-id")
}

fn generate_dummy_frame_packet_with_id(device_id: &str) -> FramePacket {
    let device_info = DeviceInformation {
        id: device_id.to_string(),
        name: "test-device".to_string(),
        device_type: DeviceType::Camera,
        device_status: DeviceStatus::Available,
    };
    let frame_info = FramePacketInformation {
        device_info,
        rx_timestamp: None,
        tx_timestamp: None,
        frame_shape: vec![2, 2],
        pixel_format: None,
        sample_rate: None,
        capture_timestamp: None,
    };
    FramePacket::new(frame_info, vec![9u8, 8, 7, 6].into_boxed_slice())
}

#[test]
fn frame_published_to_hub_reaches_access_client() {
    let context = zmq::Context::new();
    let producer_endpoint = "tcp://127.0.0.1:16101".to_string();
    let consumer_endpoint = "tcp://127.0.0.1:16102".to_string();

    let mut hub = Proxy::new_endpoints(
        producer_endpoint.clone(),
        consumer_endpoint.clone(),
        10,
        1024,
    );
    hub.start(&context).unwrap();

    let mut client = AccessClient::new_endpoint(consumer_endpoint, 10, 1024);
    client.connect().unwrap();

    let mut producer = Sender::new_endpoint(producer_endpoint, 10, 1024);
    producer.start(&context).unwrap();

    sleep(Duration::from_millis(500));

    let in_fp = generate_dummy_frame_packet();
    producer.send(in_fp.clone(), zmq::DONTWAIT).unwrap();

    let received = client
        .next_frame(Duration::from_secs(2))
        .unwrap()
        .expect("expected a frame forwarded through the hub within the timeout");

    assert_eq!(received.data, in_fp.data);
    assert_eq!(
        received.frame_info.device_info,
        in_fp.frame_info.device_info
    );
    assert_eq!(
        received.frame_info.frame_shape,
        in_fp.frame_info.frame_shape
    );

    client.disconnect().unwrap();
    producer.stop().unwrap();
    hub.stop().unwrap();
}

#[test]
fn frames_from_two_producers_reach_access_client() {
    let context = zmq::Context::new();
    let producer_a_endpoint = "tcp://127.0.0.1:16111".to_string();
    let producer_b_endpoint = "tcp://127.0.0.1:16112".to_string();
    let consumer_endpoint = "tcp://127.0.0.1:16113".to_string();

    let mut hub = Proxy::new_endpoints(
        producer_a_endpoint.clone(),
        consumer_endpoint.clone(),
        10,
        1024,
    );
    hub.add_from_endpoint(producer_b_endpoint.clone());
    hub.start(&context).unwrap();

    let mut client = AccessClient::new_endpoint(consumer_endpoint, 10, 1024);
    client.connect().unwrap();

    let mut producer_a = Sender::new_endpoint(producer_a_endpoint, 10, 1024);
    producer_a.start(&context).unwrap();
    let mut producer_b = Sender::new_endpoint(producer_b_endpoint, 10, 1024);
    producer_b.start(&context).unwrap();

    sleep(Duration::from_millis(500));

    producer_a
        .send(
            generate_dummy_frame_packet_with_id("producer-a"),
            zmq::DONTWAIT,
        )
        .unwrap();
    producer_b
        .send(
            generate_dummy_frame_packet_with_id("producer-b"),
            zmq::DONTWAIT,
        )
        .unwrap();

    let mut received_ids = HashSet::new();
    for _ in 0..10 {
        if let Some(frame) = client.next_frame(Duration::from_secs(1)).unwrap() {
            received_ids.insert(frame.frame_info.device_info.id);
        }
        if received_ids.len() == 2 {
            break;
        }
    }

    assert!(
        received_ids.contains("producer-a"),
        "missing frames from producer-a: {:?}",
        received_ids
    );
    assert!(
        received_ids.contains("producer-b"),
        "missing frames from producer-b: {:?}",
        received_ids
    );

    client.disconnect().unwrap();
    producer_a.stop().unwrap();
    producer_b.stop().unwrap();
    hub.stop().unwrap();
}
