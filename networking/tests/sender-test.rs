use std::thread::sleep;
use std::time::Duration;

use anyhow::Error;
use networking::{Receiver, Sender};
use shared::{
    ConnectionStatus, DeviceInformation, DeviceStatus, DeviceType, FramePacket,
    FramePacketInformation,
};

fn frame_packet(payload: Vec<u8>) -> FramePacket {
    let frame_info = FramePacketInformation {
        device_info: DeviceInformation {
            id: "sender-test".to_string(),
            name: "sender-test".to_string(),
            device_type: DeviceType::Camera,
            device_status: DeviceStatus::Available,
        },
        rx_timestamp: None,
        tx_timestamp: None,
        frame_shape: vec![payload.len() as u32],
    };
    FramePacket::new(frame_info, payload.into_boxed_slice())
}

fn raw_subscriber(context: &zmq::Context, endpoint: &str) -> zmq::Socket {
    let subscriber = context.socket(zmq::SUB).unwrap();
    subscriber.set_rcvtimeo(2000).unwrap();
    subscriber.set_subscribe(&[]).unwrap();
    subscriber.connect(endpoint).unwrap();
    subscriber
}

#[test]
fn sender_reports_available_before_start_and_closed_after_stop() {
    let context = zmq::Context::new();
    let mut sender = Sender::new_endpoint("inproc://sender-lifecycle", 10, 1024);
    assert_eq!(sender.connection_status(), &ConnectionStatus::Available);

    sender.start(&context).unwrap();
    assert_eq!(sender.connection_status(), &ConnectionStatus::Active);

    sender.stop().unwrap();
    assert_eq!(sender.connection_status(), &ConnectionStatus::Closed);
}

#[test]
fn send_before_start_returns_error() {
    let mut sender = Sender::new_endpoint("inproc://sender-not-started", 10, 1024);
    assert!(sender.send(frame_packet(vec![1, 2, 3]), zmq::DONTWAIT).is_err());
}

#[test]
fn start_with_invalid_endpoint_returns_error() {
    let context = zmq::Context::new();
    let mut sender = Sender::new_endpoint("not-a-valid-endpoint", 10, 1024);
    assert!(sender.start(&context).is_err());
}

#[test]
fn data_chunk_size_reports_constructor_value() {
    let sender = Sender::new_endpoint("inproc://sender-chunk-getter", 10, 2048);
    assert_eq!(sender.data_chunk_size(), 2048);
}

#[test]
#[should_panic(expected = "Data chunk size must be greater than 0.")]
fn send_with_zero_chunk_size_panics() {
    let mut sender = Sender::new_endpoint("inproc://sender-zero-chunk", 10, 0);
    let _ = sender.send(frame_packet(vec![1]), zmq::DONTWAIT);
}

#[test]
fn send_numbers_frames_with_increasing_sequence() {
    let context = zmq::Context::new();
    let endpoint = "inproc://sender-sequence";
    let mut sender = Sender::new_endpoint(endpoint, 10, 1024);
    sender.start(&context).unwrap();
    let subscriber = raw_subscriber(&context, endpoint);
    sleep(Duration::from_millis(100));

    for _ in 0..3 {
        sender.send(frame_packet(vec![7; 4]), zmq::DONTWAIT).unwrap();
    }

    for expected in 0u64..3 {
        let parts = subscriber.recv_multipart(0).unwrap();
        let sequence = u64::from_le_bytes(<[u8; 8]>::try_from(parts[1].as_slice()).unwrap());
        assert_eq!(sequence, expected);
    }

    sender.stop().unwrap();
}

#[test]
fn send_splits_payload_into_data_chunk_size_parts() {
    let context = zmq::Context::new();
    let endpoint = "inproc://sender-chunks";
    let mut sender = Sender::new_endpoint(endpoint, 10, 1024);
    sender.start(&context).unwrap();
    let subscriber = raw_subscriber(&context, endpoint);
    sleep(Duration::from_millis(100));

    sender.send(frame_packet(vec![0u8; 3000]), zmq::DONTWAIT).unwrap();

    let parts = subscriber.recv_multipart(0).unwrap();
    assert_eq!(parts.len(), 2 + 3, "header, sequence and three data chunks");
    let chunk_lengths: Vec<usize> = parts[2..].iter().map(|part| part.len()).collect();
    assert_eq!(chunk_lengths, vec![1024, 1024, 952]);

    sender.stop().unwrap();
}

#[test]
fn send_stamps_tx_timestamp_on_the_wire() {
    let context = zmq::Context::new();
    let endpoint = "inproc://sender-tx-timestamp";
    let mut sender = Sender::new_endpoint(endpoint, 10, 1024);
    sender.start(&context).unwrap();
    let subscriber = raw_subscriber(&context, endpoint);
    sleep(Duration::from_millis(100));

    sender.send(frame_packet(vec![1, 2]), zmq::DONTWAIT).unwrap();

    let parts = subscriber.recv_multipart(0).unwrap();
    let info = FramePacketInformation::deserialize(&parts[0]).unwrap();
    assert!(info.tx_timestamp.is_some());
    assert!(info.rx_timestamp.is_none());

    sender.stop().unwrap();
}

#[test]
fn chunked_payload_is_reassembled_by_receiver() -> Result<(), Error> {
    let context = zmq::Context::new();
    let endpoint = "inproc://sender-reassembly";
    let mut sender = Sender::new_endpoint(endpoint, 10, 1024);
    let mut receiver = Receiver::new_endpoint(endpoint, 10, 1024);
    sender.start(&context)?;
    receiver.start(&context)?;
    sleep(Duration::from_millis(200));

    let payload: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
    sender.send(frame_packet(payload.clone()), zmq::DONTWAIT)?;

    sleep(Duration::from_millis(100));
    let packet = receiver
        .receive(zmq::DONTWAIT)?
        .expect("the chunked frame should be received");
    assert_eq!(packet.data.as_ref(), payload.as_slice());
    assert_eq!(receiver.dropped_frames(), 0);

    receiver.stop()?;
    sender.stop()?;
    Ok(())
}
