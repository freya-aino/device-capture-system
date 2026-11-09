use std::time::{Duration, SystemTime};

use preprocess::{ImageFormat, PreprocessConfig, Preprocessor, decode_payload};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

const JPEG: &[u8] = include_bytes!("fixtures/two-tone.jpg");

/// 16x8 JPEG: left half BGR (40, 80, 160), right half grey 200. Quality 100, so values are within a
/// few levels of the source.
#[test]
fn mjpg_fixture_decodes_to_rgb() {
    let image = decode_payload(JPEG, "MJPG", &[16, 8]).unwrap();
    assert_eq!(
        (image.width, image.height, image.format),
        (16, 8, ImageFormat::Rgb)
    );
    let close = |got: &[u8], want: [u8; 3]| got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 4);
    assert!(
        close(image.pixel(2, 4), [160, 80, 40]),
        "{:?}",
        image.pixel(2, 4)
    );
    assert!(
        close(image.pixel(13, 4), [200, 200, 200]),
        "{:?}",
        image.pixel(13, 4)
    );
}

fn packet(
    device: &str,
    device_type: DeviceType,
    pixel_format: Option<&str>,
    shape: Vec<u32>,
    data: Vec<u8>,
) -> FramePacket {
    let capture = SystemTime::UNIX_EPOCH + Duration::from_millis(1234);
    FramePacket::new(
        FramePacketInformation {
            device_info: DeviceInformation {
                id: device.to_string(),
                name: "test".to_string(),
                device_type,
                device_status: DeviceStatus::Running,
            },
            rx_timestamp: None,
            tx_timestamp: Some(capture + Duration::from_millis(5)),
            frame_shape: shape,
            pixel_format: pixel_format.map(str::to_string),
            sample_rate: None,
            capture_timestamp: Some(capture),
        },
        data.into_boxed_slice(),
    )
}

fn preprocessor(toml: &str) -> Preprocessor {
    Preprocessor::new(PreprocessConfig::from_toml(toml).unwrap())
}

#[test]
fn processed_frame_keeps_its_header_and_gets_the_new_shape() {
    let p = preprocessor(
        r#"
        [[device]]
        id = "cam"
        steps = [{ rotate = 90 }, "grayscale"]
        "#,
    );
    let input = packet(
        "cam",
        DeviceType::Camera,
        Some("MJPG"),
        vec![16, 8],
        JPEG.to_vec(),
    );
    let out = p.process(&input).unwrap().expect("cam has a pipeline");

    assert_eq!(out.frame_info.frame_shape, vec![8, 16]);
    assert_eq!(out.frame_info.pixel_format.as_deref(), Some("GREY"));
    assert_eq!(out.data.len(), 8 * 16);
    assert_eq!(
        out.frame_info.capture_timestamp,
        input.frame_info.capture_timestamp
    );
    assert_eq!(out.frame_info.tx_timestamp, input.frame_info.tx_timestamp);
    assert_eq!(out.frame_info.device_info, input.frame_info.device_info);
}

#[test]
fn yuyv_frame_is_processed() {
    let p = preprocessor(
        r#"
        [[device]]
        id = "cam"
        steps = [{ brightness = 10 }]
        "#,
    );
    let input = packet(
        "cam",
        DeviceType::Camera,
        Some("YUYV"),
        vec![2, 1],
        vec![16, 128, 235, 128],
    );
    let out = p.process(&input).unwrap().unwrap();
    assert_eq!(out.frame_info.pixel_format.as_deref(), Some("RGB3"));
    assert_eq!(&*out.data, &[10, 10, 10, 255, 255, 255]);
}

#[test]
fn frames_without_a_pipeline_are_skipped() {
    let p = preprocessor(
        r#"
        [[device]]
        id = "cam"
        steps = []
        "#,
    );
    let other = packet(
        "other",
        DeviceType::Camera,
        Some("YUYV"),
        vec![2, 1],
        vec![0; 4],
    );
    let mic = packet("cam", DeviceType::Microphone, None, vec![4, 1], vec![0; 8]);
    assert!(p.process(&other).unwrap().is_none());
    assert!(p.process(&mic).unwrap().is_none());
}

#[test]
fn a_transform_that_does_not_fit_is_an_error() {
    let p = preprocessor(
        r#"
        [[device]]
        id = "cam"
        steps = [{ crop = { x = 0, y = 0, width = 64, height = 64 } }]
        "#,
    );
    let input = packet(
        "cam",
        DeviceType::Camera,
        Some("MJPG"),
        vec![16, 8],
        JPEG.to_vec(),
    );
    assert!(p.process(&input).is_err());
}
