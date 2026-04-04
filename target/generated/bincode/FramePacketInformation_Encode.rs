impl :: bincode :: Encode for FramePacketInformation
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.device_info, encoder) ?; ::
        bincode :: Encode :: encode(&self.rx_timestamp, encoder) ?; :: bincode
        :: Encode :: encode(&self.tx_timestamp, encoder) ?; :: bincode ::
        Encode :: encode(&self.frame_shape, encoder) ?; core :: result ::
        Result :: Ok(())
    }
}