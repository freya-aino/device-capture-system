//! The forwarding loop against a loopback hub: a buffer sink in the hardware-free test, and a real
//! `v4l2loopback` device in the ignored one.

use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;

use live_sink::{
    Conversion, FrameSink, LiveConfig, LiveSource, OutputFormat, forward, forward_with,
};
use networking::{Proxy, Sender};
use shared::{DeviceInformation, DeviceStatus, DeviceType, FramePacket, FramePacketInformation};

fn frame(id: &str, format: &str, shape: &[u32], data: Vec<u8>) -> FramePacket {
    FramePacket::new(
        FramePacketInformation {
            device_info: DeviceInformation {
                id: id.into(),
                name: id.into(),
                device_type: DeviceType::Camera,
                device_status: DeviceStatus::Running,
            },
            rx_timestamp: None,
            tx_timestamp: None,
            frame_shape: shape.to_vec(),
            pixel_format: Some(format.into()),
            sample_rate: None,
            capture_timestamp: None,
        },
        data.into_boxed_slice(),
    )
}

fn config(port: u16, sink: &str) -> LiveConfig {
    LiveConfig {
        hub_ip: Ipv4Addr::LOCALHOST,
        source: LiveSource::Raw,
        hub_port: port,
        device: "cam".into(),
        sink: sink.into(),
    }
}

#[derive(Default)]
struct BufferSink(Arc<Mutex<Vec<Vec<u8>>>>);

impl FrameSink for BufferSink {
    fn write_frame(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.0.lock().unwrap().push(data.to_vec());
        Ok(())
    }
}

#[test]
fn forwards_only_the_selected_device_and_fixes_the_format_from_the_first_frame() {
    let context = zmq::Context::new();
    let mut hub = Proxy::new_endpoints(
        "tcp://127.0.0.1:17401".to_string(),
        "tcp://127.0.0.1:17402".to_string(),
        64,
        1024,
    );
    hub.start(&context).unwrap();
    let mut producer = Sender::new_endpoint("tcp://127.0.0.1:17401".to_string(), 64, 1024);
    producer.start(&context).unwrap();

    let written = Arc::new(Mutex::new(Vec::new()));
    let plans = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let forwarder = {
        let (written, plans, stop) = (written.clone(), plans.clone(), stop.clone());
        spawn(move || {
            forward_with(&config(17402, "/dev/null"), &stop, move |plan| {
                plans.lock().unwrap().push(plan.clone());
                Ok(Box::new(BufferSink(written.clone())) as Box<dyn FrameSink>)
            })
        })
    };
    sleep(Duration::from_millis(500));

    for i in 0..5u8 {
        producer
            .send(frame("cam", "RGB3", &[2, 1], vec![i; 6]), zmq::DONTWAIT)
            .unwrap();
        producer
            .send(frame("other", "YUYV", &[2, 1], vec![9; 4]), zmq::DONTWAIT)
            .unwrap();
        sleep(Duration::from_millis(30));
    }
    sleep(Duration::from_millis(300));
    stop.store(true, Ordering::SeqCst);
    let stats = forwarder.join().unwrap().unwrap();
    producer.stop().unwrap();
    hub.stop().unwrap();

    let plans = plans.lock().unwrap();
    assert_eq!(plans.len(), 1, "the sink is opened once");
    assert_eq!(
        (plans[0].output, plans[0].conversion),
        (OutputFormat::Yuyv, Conversion::RgbToYuyv)
    );
    let written = written.lock().unwrap();
    assert_eq!(written.len() as u64, stats.written);
    assert!(stats.written + stats.skipped == 5, "{stats:?}");
    assert!(stats.written >= 1);
    assert!(
        written.iter().all(|f| f.len() == 4),
        "RGB3 2x1 became YUYV: {written:?}"
    );
    // The last frame is RGB (4, 4, 4): luma 16 + ((66 + 129 + 25) * 4 + 128 >> 8) = 19.
    assert_eq!(written.last().unwrap(), &[19, 128, 19, 128]);
}

#[test]
fn a_changed_stream_is_an_error() {
    let context = zmq::Context::new();
    let mut hub = Proxy::new_endpoints(
        "tcp://127.0.0.1:17411".to_string(),
        "tcp://127.0.0.1:17412".to_string(),
        64,
        1024,
    );
    hub.start(&context).unwrap();
    let mut producer = Sender::new_endpoint("tcp://127.0.0.1:17411".to_string(), 64, 1024);
    producer.start(&context).unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let forwarder = {
        let stop = stop.clone();
        spawn(move || {
            forward_with(&config(17412, "/dev/null"), &stop, |_| {
                Ok(Box::new(BufferSink::default()) as Box<dyn FrameSink>)
            })
        })
    };
    sleep(Duration::from_millis(500));
    producer
        .send(frame("cam", "YUYV", &[2, 1], vec![0; 4]), zmq::DONTWAIT)
        .unwrap();
    sleep(Duration::from_millis(300));
    producer
        .send(frame("cam", "YUYV", &[4, 1], vec![0; 8]), zmq::DONTWAIT)
        .unwrap();
    sleep(Duration::from_millis(300));
    stop.store(true, Ordering::SeqCst);
    let result = forwarder.join().unwrap();
    producer.stop().unwrap();
    hub.stop().unwrap();
    let err = result.expect_err("a changed frame size stops the sink");
    assert!(err.to_string().contains("stream changed"), "{err:#}");
}

/// Needs `sudo modprobe v4l2loopback video_nr=10 card_label=device-capture exclusive_caps=1`
/// (`LIVE_TEST_SINK` overrides the path). Forwards a YUYV pattern and reads it back through V4L2.
#[test]
#[ignore]
fn frames_round_trip_through_a_v4l2loopback_device() {
    use v4l::buffer::Type;
    use v4l::io::traits::CaptureStream;
    use v4l::video::Capture;

    let sink = std::env::var("LIVE_TEST_SINK").unwrap_or_else(|_| "/dev/video10".to_string());
    let context = zmq::Context::new();
    let mut hub = Proxy::new_endpoints(
        "tcp://127.0.0.1:17421".to_string(),
        "tcp://127.0.0.1:17422".to_string(),
        64,
        1024,
    );
    hub.start(&context).unwrap();
    let mut producer = Sender::new_endpoint("tcp://127.0.0.1:17421".to_string(), 64, 1024);
    producer.start(&context).unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let forwarder = {
        let (stop, sink) = (stop.clone(), sink.clone());
        spawn(move || forward(&config(17422, &sink), &stop))
    };
    sleep(Duration::from_millis(500));

    let (w, h) = (64u32, 32u32);
    let pattern: Vec<u8> = (0..(w * h * 2)).map(|i| (i % 251) as u8).collect();
    // Keep publishing so the reader, which opens the device after the format is set, sees frames.
    let publishing = Arc::new(AtomicBool::new(true));
    let publisher = {
        let (publishing, pattern) = (publishing.clone(), pattern.clone());
        spawn(move || {
            while publishing.load(Ordering::SeqCst) {
                producer
                    .send(
                        frame("cam", "YUYV", &[w, h], pattern.clone()),
                        zmq::DONTWAIT,
                    )
                    .unwrap();
                sleep(Duration::from_millis(33));
            }
            producer
        })
    };
    sleep(Duration::from_millis(700));

    let device = v4l::Device::with_path(&sink).expect("open the loopback device for capture");
    let format = Capture::format(&device).unwrap();
    assert_eq!((format.width, format.height), (w, h));
    assert_eq!(format.fourcc, v4l::FourCC::new(b"YUYV"));
    let mut stream = v4l::io::mmap::Stream::with_buffers(&device, Type::VideoCapture, 4)
        .expect("capture stream");
    let (buf, _meta) = stream.next().expect("a frame from the loopback device");
    assert_eq!(&buf[..pattern.len()], &pattern[..]);
    drop(stream);

    publishing.store(false, Ordering::SeqCst);
    let mut producer = publisher.join().unwrap();
    stop.store(true, Ordering::SeqCst);
    let stats = forwarder.join().unwrap().unwrap();
    assert!(stats.written > 0);
    producer.stop().unwrap();
    hub.stop().unwrap();
}
