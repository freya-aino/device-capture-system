use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use devices::V4lCameraDevice;
use shared::{Device, DeviceCommand, select_camera_config};

#[test]
#[ignore = "requires a real V4L2 capture device at /dev/video0"]
fn real_camera_streams_frames_for_a_few_seconds() {
    let mut camera = V4lCameraDevice::new("/dev/video0".to_string())
        .expect("a V4L2 capture device should be present at /dev/video0");
    let configs = camera.get_configs().unwrap();
    for config in configs.iter() {
        println!(
            "advertised {}x{} {:?} interval {:?} ({:.1} fps)",
            config.width,
            config.height,
            config.fourcc.iter().collect::<String>(),
            config.fps,
            config.frames_per_second()
        );
    }
    let config = select_camera_config(&configs, 1920, 1080, 30.0)
        .expect("camera should advertise at least one config");
    println!(
        "streaming {}x{} at {:?} fps",
        config.width, config.height, config.fps
    );

    let frames = Arc::new(AtomicUsize::new(0));
    let counter = frames.clone();
    let handle = camera
        .start(
            config,
            Box::new(move |packet| {
                assert!(!packet.data.is_empty());
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        )
        .unwrap();

    let start = Instant::now();
    sleep(Duration::from_secs(3));
    let elapsed = start.elapsed();

    if let Some(tx) = camera.command_tx.as_ref() {
        tx.send(DeviceCommand::Stop).unwrap();
    }
    handle.join().unwrap().unwrap();

    let count = frames.load(Ordering::SeqCst);
    println!(
        "received {} frames in {:.2}s ({:.1} fps)",
        count,
        elapsed.as_secs_f64(),
        count as f64 / elapsed.as_secs_f64()
    );
    assert!(count > 0, "camera delivered no frames");
}
