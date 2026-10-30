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
        frame_shape: vec![1920, 1080],
        pixel_format: None,
        sample_rate: None,
    };
    FramePacket::new(frame_info, vec![1u8, 2, 3, 4].into_boxed_slice())
}

fn generate_dummy_frame_packet_with_id(device_id: &str) -> FramePacket {
    let mut packet = generate_dummy_frame_packet();
    packet.frame_info.device_info.id = device_id.to_string();
    packet
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::{thread::sleep, time::Duration};

    use networking::{Proxy, Receiver, Sender};

    use crate::{generate_dummy_frame_packet, generate_dummy_frame_packet_with_id};

    static ENDPOINT_ID: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn fan_in_fan_out() {
        let context = zmq::Context::new();

        let id = ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
        let producer_endpoint = format!("inproc://proxy-producer-{}", id);
        let consumer_endpoint = format!("inproc://proxy-consumer-{}", id);

        let mut proxy = Proxy::new_endpoints(
            producer_endpoint.clone(),
            consumer_endpoint.clone(),
            10,
            1024,
        );
        proxy.start(&context).unwrap();

        let mut receiver = Receiver::new_endpoint(consumer_endpoint, 10, 1024);
        receiver.start(&context).unwrap();

        let mut sender = Sender::new_endpoint(producer_endpoint, 10, 1024);
        sender.start(&context).unwrap();

        // Give the XSUB/XPUB subscription forwarding time to propagate before
        // publishing, otherwise the PUB side may send before the hub has
        // forwarded the subscriber's subscription upstream.
        sleep(Duration::from_millis(500));

        let in_fp = generate_dummy_frame_packet();
        sender.send(in_fp.clone(), zmq::DONTWAIT).unwrap();

        sleep(Duration::from_millis(500));

        let rcv = receiver.receive(zmq::DONTWAIT).unwrap();
        let out_fp = rcv.expect("expected a frame packet to be forwarded through the proxy");

        assert_eq!(in_fp.data, out_fp.data);
        assert_eq!(in_fp.frame_info.device_info, out_fp.frame_info.device_info);

        sender.stop().unwrap();
        receiver.stop().unwrap();
        proxy.stop().unwrap();
    }

    #[test]
    fn fan_in_two_producers() {
        let context = zmq::Context::new();

        let id = ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
        let producer_a = format!("inproc://proxy-fan-in-a-{}", id);
        let producer_b = format!("inproc://proxy-fan-in-b-{}", id);
        let consumer_endpoint = format!("inproc://proxy-fan-in-consumer-{}", id);

        let mut proxy =
            Proxy::new_endpoints(producer_a.clone(), consumer_endpoint.clone(), 10, 1024);
        proxy.add_from_endpoint(producer_b.clone());
        proxy.start(&context).unwrap();

        let mut receiver = Receiver::new_endpoint(consumer_endpoint, 10, 1024);
        receiver.start(&context).unwrap();

        let mut sender_a = Sender::new_endpoint(producer_a, 10, 1024);
        sender_a.start(&context).unwrap();
        let mut sender_b = Sender::new_endpoint(producer_b, 10, 1024);
        sender_b.start(&context).unwrap();

        sleep(Duration::from_millis(500));

        sender_a
            .send(
                generate_dummy_frame_packet_with_id("producer-a"),
                zmq::DONTWAIT,
            )
            .unwrap();
        sender_b
            .send(
                generate_dummy_frame_packet_with_id("producer-b"),
                zmq::DONTWAIT,
            )
            .unwrap();

        sleep(Duration::from_millis(500));

        let mut received_ids = HashSet::new();
        for _ in 0..50 {
            if let Some(packet) = receiver.receive(zmq::DONTWAIT).unwrap() {
                received_ids.insert(packet.frame_info.device_info.id);
            }
            if received_ids.len() == 2 {
                break;
            }
            sleep(Duration::from_millis(20));
        }

        assert!(
            received_ids.contains("producer-a"),
            "missing frames from producer-a: {:?}",
            received_ids
        );
        assert!(
            received_ids.contains("producer-b"),
            "missing frames from producer-b: {:?}",
            received_ids
        );

        sender_a.stop().unwrap();
        sender_b.stop().unwrap();
        receiver.stop().unwrap();
        proxy.stop().unwrap();
    }
}
