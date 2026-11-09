use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use devices::V4lCameraDevice;
use shared::{CameraConfig, Device, DeviceStatus, select_camera_config};

/// Streams `config` for `seconds` and prints the delivered rate measured from the first frame,
/// so the stream start-up (STREAMON, first exposure) is not counted against the frame rate.
fn stream_and_report(camera: &mut V4lCameraDevice, config: CameraConfig, seconds: u64) -> usize {
    println!("streaming {:?}", config);
    let expected_fps = config.frames_per_second();

    let arrivals: Arc<Mutex<Vec<Instant>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = arrivals.clone();
    let started = Instant::now();
    let handle = camera
        .start(
            config,
            Box::new(move |packet| {
                assert!(!packet.data.is_empty());
                assert!(packet.frame_info.pixel_format.is_some());
                sink.lock().unwrap().push(Instant::now());
                Ok(())
            }),
        )
        .unwrap();

    sleep(Duration::from_secs(seconds));

    camera.stop().unwrap();
    handle.join().unwrap().unwrap();
    assert_eq!(camera.device_status(), &DeviceStatus::Stopped);

    let arrivals = arrivals.lock().unwrap();
    let count = arrivals.len();
    if let (Some(first), Some(last)) = (arrivals.first(), arrivals.last()) {
        let span = last.duration_since(*first).as_secs_f64();
        let steady_fps = if span > 0.0 {
            (count - 1) as f64 / span
        } else {
            0.0
        };
        let gaps: Vec<f64> = arrivals
            .windows(2)
            .map(|w| w[1].duration_since(w[0]).as_secs_f64() * 1000.0)
            .collect();
        let max_gap = gaps.iter().cloned().fold(0.0, f64::max);
        println!(
            "  first frame after {:.0} ms; {} frames over {:.2} s = {:.1} fps (expected {:.1}); max gap {:.0} ms",
            first.duration_since(started).as_secs_f64() * 1000.0,
            count,
            span,
            steady_fps,
            expected_fps,
            max_gap
        );
    } else {
        println!("  no frames");
    }
    count
}

#[test]
#[ignore = "requires a real V4L2 capture device at /dev/video0"]
fn real_camera_streams_frames_for_a_few_seconds() {
    let mut camera = V4lCameraDevice::new("/dev/video0".to_string())
        .expect("a V4L2 capture device should be present at /dev/video0");
    let configs = camera.get_configs().unwrap();
    for config in configs.iter() {
        println!("advertised {:?}", config);
    }

    let config = select_camera_config(&configs, 1920, 1080, 30.0)
        .expect("camera should advertise at least one config");
    let count = stream_and_report(&mut camera, config, 3);
    assert!(count > 0, "camera delivered no frames");
}

#[test]
#[ignore = "requires a real V4L2 capture device at /dev/video0"]
fn real_camera_rate_survey() {
    let mut camera = V4lCameraDevice::new("/dev/video0".to_string())
        .expect("a V4L2 capture device should be present at /dev/video0");
    let configs = camera.get_configs().unwrap();
    // One run per advertised config, so the delivered rate can be compared across formats.
    for config in configs {
        stream_and_report(&mut camera, config, 3);
    }
}
