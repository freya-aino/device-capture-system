use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::sleep;
use std::time::Duration;

use access_client::AccessClient;
use networking::Sender;
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

static ENDPOINT_ID: AtomicUsize = AtomicUsize::new(0);

fn generate_dummy_frame_packet() -> FramePacket {
    let device_info = DeviceInformation {
        id: "test-device-id".to_string(),
        name: "test-device".to_string(),
        device_type: DeviceType::Microphone,
        device_status: DeviceStatus::Available,
    };
    let frame_info = FramePacketInformation {
        device_info,
        rx_timestamp: None,
        tx_timestamp: None,
        frame_shape: vec![512],
    };
    FramePacket::new(frame_info, vec![1u8, 2, 3].into_boxed_slice())
}

#[test]
fn next_frame_returns_published_frame() {
    let context = zmq::Context::new();
    let id = ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
    let endpoint = format!("inproc://access-client-test-{}", id);

    let mut client = AccessClient::new_endpoint(endpoint.clone(), 10, 1024);
    client.connect().unwrap();

    let mut sender = Sender::new_endpoint(endpoint, 10, 1024);
    sender.start(&context).unwrap();

    sleep(Duration::from_millis(300));

    let in_fp = generate_dummy_frame_packet();
    sender.send(in_fp.clone(), zmq::DONTWAIT, 1024).unwrap();

    let received = client
        .next_frame(Duration::from_secs(2))
        .unwrap()
        .expect("expected a frame within the timeout");

    assert_eq!(received.data, in_fp.data);
    assert_eq!(
        received.frame_info.device_info,
        in_fp.frame_info.device_info
    );

    client.disconnect().unwrap();
    sender.stop().unwrap();
}

#[test]
fn next_frame_times_out_when_nothing_is_published() {
    let id = ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
    let endpoint = format!("inproc://access-client-timeout-test-{}", id);

    let mut client = AccessClient::new_endpoint(endpoint, 10, 1024);
    client.connect().unwrap();

    let result = client.next_frame(Duration::from_millis(200)).unwrap();

    assert!(result.is_none());
    client.disconnect().unwrap();
}
