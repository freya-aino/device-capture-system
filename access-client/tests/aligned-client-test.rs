use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use access_client::{AccessClient, AlignedClient};
use networking::{Proxy, Sender};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn generate_frame_packet_with_id(device_id: &str) -> FramePacket {
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
    };
    FramePacket::new(frame_info, vec![9u8, 8, 7, 6].into_boxed_slice())
}

#[test]
fn aligned_client_groups_frames_from_two_producers() {
    let context = zmq::Context::new();
    let producer_a_endpoint = "tcp://127.0.0.1:16121".to_string();
    let producer_b_endpoint = "tcp://127.0.0.1:16122".to_string();
    let consumer_endpoint = "tcp://127.0.0.1:16123".to_string();

    let mut hub = Proxy::new_endpoints(
        producer_a_endpoint.clone(),
        consumer_endpoint.clone(),
        10,
        1024,
    );
    hub.add_from_endpoint(producer_b_endpoint.clone());
    hub.start(&context).unwrap();

    let mut client = AlignedClient::new(
        AccessClient::new_endpoint(consumer_endpoint, 10, 1024),
        vec!["producer-a".to_string(), "producer-b".to_string()],
        Duration::from_millis(500),
    );
    client.connect().unwrap();

    let mut producer_a = Sender::new_endpoint(producer_a_endpoint, 10, 1024);
    producer_a.start(&context).unwrap();
    let mut producer_b = Sender::new_endpoint(producer_b_endpoint, 10, 1024);
    producer_b.start(&context).unwrap();

    sleep(Duration::from_millis(500));

    producer_a
        .send(generate_frame_packet_with_id("producer-a"), zmq::DONTWAIT)
        .unwrap();
    producer_b
        .send(generate_frame_packet_with_id("producer-b"), zmq::DONTWAIT)
        .unwrap();

    let group = client
        .next_group(Duration::from_secs(3))
        .unwrap()
        .expect("expected an aligned group from both producers within the timeout");

    assert_eq!(group.samples.len(), 2);
    let ids: HashSet<String> = group.samples.iter().map(|s| s.device_id.clone()).collect();
    assert!(ids.contains("producer-a"), "missing producer-a: {:?}", ids);
    assert!(ids.contains("producer-b"), "missing producer-b: {:?}", ids);
    assert!(group.skew <= Duration::from_millis(500));
    for sample in &group.samples {
        assert_eq!(sample.payload.frame_info.device_info.id, sample.device_id);
        assert_eq!(sample.payload.data.len(), 4);
    }

    client.disconnect().unwrap();
    producer_a.stop().unwrap();
    producer_b.stop().unwrap();
    hub.stop().unwrap();
}

#[test]
fn aligned_client_emits_nothing_while_a_producer_is_silent() {
    let context = zmq::Context::new();
    let producer_endpoint = "tcp://127.0.0.1:16124".to_string();
    let consumer_endpoint = "tcp://127.0.0.1:16125".to_string();

    let mut hub = Proxy::new_endpoints(
        producer_endpoint.clone(),
        consumer_endpoint.clone(),
        10,
        1024,
    );
    hub.start(&context).unwrap();

    let mut client = AlignedClient::new(
        AccessClient::new_endpoint(consumer_endpoint, 10, 1024),
        vec!["producer-a".to_string(), "producer-b".to_string()],
        Duration::from_millis(500),
    );
    client.connect().unwrap();

    let mut producer = Sender::new_endpoint(producer_endpoint, 10, 1024);
    producer.start(&context).unwrap();

    sleep(Duration::from_millis(500));

    producer
        .send(generate_frame_packet_with_id("producer-a"), zmq::DONTWAIT)
        .unwrap();

    let group = client.next_group(Duration::from_secs(1)).unwrap();
    assert!(group.is_none(), "no group should form without producer-b");

    client.disconnect().unwrap();
    producer.stop().unwrap();
    hub.stop().unwrap();
}
