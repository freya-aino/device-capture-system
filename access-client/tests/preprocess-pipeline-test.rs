use std::thread::sleep;
use std::time::{Duration, SystemTime};

use access_client::AccessClient;
use networking::{FrameRelay, Proxy, Sender};
use preprocess::{PreprocessConfig, Preprocessor};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn yuyv_frame(device_id: &str) -> FramePacket {
    let frame_info = FramePacketInformation {
        device_info: DeviceInformation {
            id: device_id.to_string(),
            name: "test-camera".to_string(),
            device_type: DeviceType::Camera,
            device_status: DeviceStatus::Running,
        },
        rx_timestamp: None,
        tx_timestamp: None,
        // 4x2 YUYV: top row black, bottom row white, neutral chroma.
        frame_shape: vec![4, 2],
        pixel_format: Some("YUYV".to_string()),
        sample_rate: None,
        capture_timestamp: Some(SystemTime::UNIX_EPOCH + Duration::from_millis(4242)),
    };
    let data = vec![
        16, 128, 16, 128, 16, 128, 16, 128, //
        235, 128, 235, 128, 235, 128, 235, 128,
    ];
    FramePacket::new(frame_info, data.into_boxed_slice())
}

/// The hub layout with `HUB_PREPROCESS_PORT` set: the proxy forwards raw frames and a `FrameRelay`
/// running a `Preprocessor` publishes processed ones on a separate port.
#[test]
fn hub_publishes_processed_frames_beside_the_raw_stream() {
    let context = zmq::Context::new();
    let producer = "tcp://127.0.0.1:16531".to_string();
    let raw_out = "tcp://127.0.0.1:16532".to_string();
    let processed_out = "tcp://127.0.0.1:16533".to_string();

    let mut hub = Proxy::new_endpoints(producer.clone(), raw_out.clone(), 10, 1024);
    hub.start(&context).unwrap();
    let config = PreprocessConfig::from_toml(
        r#"
        [[device]]
        id = "cam"
        steps = [{ rotate = 90 }, "grayscale"]
        "#,
    )
    .unwrap();
    let preprocessor = Preprocessor::new(config);
    let mut relay = FrameRelay::new(
        vec![producer.clone()],
        processed_out.clone(),
        1024,
        Box::new(move |packet| preprocessor.process(&packet)),
    );
    relay.start(&context).unwrap();

    let mut raw_client = AccessClient::new_endpoint(raw_out, 10, 1024);
    raw_client.connect().unwrap();
    let mut processed_client = AccessClient::new_endpoint(processed_out, 10, 1024);
    processed_client.connect().unwrap();

    let mut sender = Sender::new_endpoint(producer, 10, 1024);
    sender.start(&context).unwrap();
    sleep(Duration::from_millis(500));
    let input = yuyv_frame("cam");
    sender.send(input.clone(), zmq::DONTWAIT).unwrap();

    let raw = raw_client
        .next_frame(Duration::from_secs(2))
        .unwrap()
        .expect("raw frame through the proxy");
    assert_eq!(raw.data, input.data, "the raw stream is unchanged");

    let processed = processed_client
        .next_frame(Duration::from_secs(2))
        .unwrap()
        .expect("processed frame through the relay");
    assert_eq!(processed.frame_info.frame_shape, vec![2, 4]);
    assert_eq!(processed.frame_info.pixel_format.as_deref(), Some("GREY"));
    assert_eq!(
        processed.frame_info.capture_timestamp,
        input.frame_info.capture_timestamp
    );
    // Rotated clockwise, the white bottom row becomes the left column.
    assert_eq!(processed.data.as_ref(), &[255u8, 0, 255, 0, 255, 0, 255, 0]);

    raw_client.disconnect().unwrap();
    processed_client.disconnect().unwrap();
    sender.stop().unwrap();
    relay.stop().unwrap();
    hub.stop().unwrap();
}
