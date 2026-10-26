use std::thread::sleep;
use std::time::Duration;

use access_client::AccessClient;
use networking::{Proxy, Sender};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn generate_dummy_frame_packet() -> FramePacket {
    let device_info = DeviceInformation {
        id: "test-device-id".to_string(),
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
    producer.send(in_fp.clone(), zmq::DONTWAIT, 1024).unwrap();

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
