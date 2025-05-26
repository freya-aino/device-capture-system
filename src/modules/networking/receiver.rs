use zmq::SUB;
use zmq::Error::EAGAIN;


pub struct Receiver(Connection);


impl Receiver {
    pub fn new(host_address: Ipv4Addr, port: u16, queue_size: u32) -> Self {
        Receiver(Connection::new(host_address, port, queue_size))
    }

    pub fn initialize(&mut self, context: &Context) -> Result<(), Error> {
        self.0.initialize(context, SUB)
    }

    pub fn receive(&mut self, zmq_flags: i32) -> Result<Option<FramePacket>, Error> {
        assert!(
            self.0.status != ConnectionStatus::Closed,
            "Trying to receive while connection is closed."
        );
        assert!(
            self.0.status != ConnectionStatus::Created,
            "Trying to receive while connection is not initialized."
        );
        assert!(
            !self.0.socket.is_none(),
            "Trying to receive while socket is None (Status is not correct, this might hint at a previous function having exited unexpetedly)."
        );

        if self.0.status == ConnectionStatus::Paused {
            println!("Calling receive while connection is paused...");
            return Ok(None);
        }

        let socket = self.0.get_socket()?;

        match socket.recv_multipart(zmq_flags) {
            Ok(parts) => {
                assert!(parts.len() >= 2, "Expected 2 or more parts in the message.");

                let rcv_ts = SystemTime::now();

                let frame_information = FramePacketInformation::deserialize(&parts[0]).unwrap();
                let data = parts[1..].concat().to_vec();

                self.0.set_status(ConnectionStatus::Active);
                self.0.update_stats(data.len() as u64);

                return Ok(Some(FramePacket::new(
                    rcv_ts,
                    frame_information.frame_shape,
                    data,
                )));
            }
            Err(e) if e == EAGAIN => {
                // No message available, continue waiting
                return Ok(None);
            }
            Err(e) => {
                self.0.set_status(ConnectionStatus::Error);
                return Err(Error::msg(format!("Error receiving message: {}", e)));
            }
        }
    }
}

