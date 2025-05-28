use shared::{DeviceInformation, DeviceType, FramePacket, FramePacketInformation};
use std::time::SystemTime;

fn generate_dummy_frame(size: usize) -> Vec<u8> {
    vec![0u8; size]
}

fn generate_dummy_frame_packet_info(size: usize) -> FramePacketInformation {
    let device_info = DeviceInformation {
        id: 0,
        name: "test-device".to_string(),
        device_type: DeviceType::Camera,
    };
    FramePacketInformation {
        device_info: device_info,
        rx_timestamp: None,
        tx_timestamp: None,
        frame_shape: vec![size as u32],
    }
}

fn generate_dummy_frame_packet() -> FramePacket {
    let frame = generate_dummy_frame(1920 * 1080 * 3).into_boxed_slice();
    let frame_packet_info = generate_dummy_frame_packet_info(1920 * 1080 * 3);
    FramePacket::new(frame_packet_info, frame)
}

#[cfg(test)]
mod tests {
    use std::{net::Ipv4Addr, thread::sleep, time::Duration};

    use networking::{Receiver, Sender};
    use portpicker::pick_unused_port;

    use crate::{generate_dummy_frame, generate_dummy_frame_packet};

    #[test]
    fn send_and_receive() {
        let context = zmq::Context::new();

        let port = pick_unused_port().unwrap();
        let mut sender = Sender::new(Ipv4Addr::new(127, 0, 0, 1), port, 10);
        let mut receiver = Receiver::new(Ipv4Addr::new(127, 0, 0, 1), port, 10);

        sender.initialize(&context).unwrap();
        receiver.initialize(&context).unwrap();

        let in_fp = generate_dummy_frame_packet();
        sender
            .send(in_fp.clone(), zmq::DONTWAIT, 1024 * 100)
            .unwrap();

        sleep(Duration::from_millis(500));

        let rcv = receiver.receive(zmq::DONTWAIT);
        let val = rcv.expect("not able to receive a message");
        let out_fp = val.expect("frame packet is none");
        assert!(in_fp.data == out_fp.data, "data is not the same");
        assert!(
            in_fp.frame_info.device_info == out_fp.frame_info.device_info,
            "device info is not the same\n\n{:?}\n!=\n{:?}",
            in_fp.frame_info.device_info,
            out_fp.frame_info.device_info
        );
        assert!(
            in_fp.frame_info.frame_shape == out_fp.frame_info.frame_shape,
            "frame shape is not the same\n\n{:?}\n!=\n{:?}",
            in_fp.frame_info.frame_shape,
            out_fp.frame_info.frame_shape
        );
    }
}
