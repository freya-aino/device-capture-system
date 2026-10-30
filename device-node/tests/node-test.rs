use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Error, Result};
use device_node::DeviceNode;
use networking::Receiver;
use shared::{
    ConnectionStatus, Device, DeviceConfig, DeviceInformation, DeviceStatus, DeviceType,
    FramePacket, FramePacketInformation,
};

static ENDPOINT_ID: AtomicUsize = AtomicUsize::new(0);

/// A fake, hardware-free stand-in for a real camera/microphone device. Instead of
/// touching v4l/alsa it immediately emits one dummy frame through the callback when
/// started, which is enough to exercise `DeviceNode`'s wiring end to end.
struct MockDevice {
    info: DeviceInformation,
}

#[derive(Clone)]
struct MockDeviceConfig;
impl DeviceConfig for MockDeviceConfig {}

impl MockDevice {
    fn new() -> Self {
        MockDevice {
            info: DeviceInformation {
                id: "mock-id".to_string(),
                name: "mock-device".to_string(),
                device_type: DeviceType::Camera,
                device_status: DeviceStatus::Available,
            },
        }
    }
}

impl Device for MockDevice {
    type Config = MockDeviceConfig;

    fn id(&self) -> &str {
        &self.info.id
    }
    fn name(&self) -> &str {
        &self.info.name
    }
    fn device_type(&self) -> &DeviceType {
        &self.info.device_type
    }
    fn device_status(&self) -> &DeviceStatus {
        &self.info.device_status
    }

    fn get_configs(&self) -> Result<Vec<Self::Config>, Error> {
        Ok(vec![MockDeviceConfig])
    }

    fn start(
        &mut self,
        _config: Self::Config,
        callback: Box<dyn Fn(FramePacket) -> Result<(), Error> + Send + 'static>,
    ) -> Result<std::thread::JoinHandle<Result<(), Error>>, Error> {
        let device_info = self.info.clone();
        let handle = std::thread::spawn(move || -> Result<(), Error> {
            // Give the SUB socket time to finish connecting/subscribing before
            // publishing, otherwise PUB/SUB may silently drop this "late joiner" frame.
            sleep(Duration::from_millis(300));
            let packet = FramePacket::new(
                FramePacketInformation {
                    device_info,
                    rx_timestamp: None,
                    tx_timestamp: None,
                    frame_shape: vec![1, 1],
                    pixel_format: None,
                    sample_rate: None,
                },
                vec![42u8].into_boxed_slice(),
            );
            callback(packet)
        });
        Ok(handle)
    }

    fn stop(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[test]
fn device_node_publishes_frames_from_its_device() {
    let context = zmq::Context::new();
    let id = ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
    let endpoint = format!("inproc://device-node-test-{}", id);

    let mut node = DeviceNode::new_endpoint(MockDevice::new(), endpoint.clone(), 10, 1024);

    let mut receiver = Receiver::new_endpoint(endpoint, 10, 1024);
    receiver.start(&context).unwrap();

    node.start(MockDeviceConfig, &context).unwrap();

    sleep(Duration::from_millis(800));

    let (device_status, connection_status) = node.status().unwrap();
    assert_eq!(device_status, DeviceStatus::Available);
    assert_eq!(connection_status, ConnectionStatus::Active);

    let received = receiver
        .receive(zmq::DONTWAIT)
        .unwrap()
        .expect("expected a frame published by the mock device");

    assert_eq!(received.data.as_ref(), &[42u8]);
    assert_eq!(received.frame_info.device_info.name, "mock-device");
}
