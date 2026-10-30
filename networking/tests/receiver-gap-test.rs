use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};

use networking::Receiver;
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacketInformation};

fn parts_with_sequence(sequence: u64) -> Vec<Vec<u8>> {
    let info = FramePacketInformation {
        device_info: DeviceInformation {
            id: "gap-device".to_string(),
            name: "gap".to_string(),
            device_type: DeviceType::Camera,
            device_status: DeviceStatus::Available,
        },
        rx_timestamp: None,
        tx_timestamp: Some(SystemTime::now()),
        frame_shape: vec![2, 2],
        pixel_format: None,
        sample_rate: None,
    };
    vec![
        info.serialize().unwrap(),
        sequence.to_le_bytes().to_vec(),
        vec![1, 2, 3, 4],
    ]
}

fn receive_frame(receiver: &mut Receiver) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if receiver.receive(zmq::DONTWAIT).unwrap().is_some() {
            return true;
        }
        sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn receiver_counts_sequence_gaps_as_dropped_frames() {
    let context = zmq::Context::new();
    let endpoint = "inproc://receiver-gap-test";

    let publisher = context.socket(zmq::PUB).unwrap();
    publisher.bind(endpoint).unwrap();

    let mut receiver = Receiver::new_endpoint(endpoint, 16, 1024);
    receiver.start(&context).unwrap();
    sleep(Duration::from_millis(300));

    publisher.send_multipart(parts_with_sequence(0), 0).unwrap();
    assert!(receive_frame(&mut receiver), "sequence 0 should arrive");
    assert_eq!(receiver.dropped_frames(), 0);

    // sequences 1 and 2 are never published, so two frames should be counted as dropped
    publisher.send_multipart(parts_with_sequence(3), 0).unwrap();
    assert!(receive_frame(&mut receiver), "sequence 3 should arrive");
    assert_eq!(receiver.dropped_frames(), 2);

    receiver.stop().unwrap();
}
