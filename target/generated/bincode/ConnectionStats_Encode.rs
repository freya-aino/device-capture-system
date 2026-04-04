impl :: bincode :: Encode for ConnectionStats
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.frames, encoder) ?; :: bincode ::
        Encode :: encode(&self.bytes, encoder) ?; :: bincode :: Encode ::
        encode(&self.current_fps, encoder) ?; :: bincode :: Encode ::
        encode(&self.current_bitrate, encoder) ?; :: bincode :: Encode ::
        encode(&self.current_latency, encoder) ?; :: bincode :: Encode ::
        encode(&self.start_time, encoder) ?; :: bincode :: Encode ::
        encode(&self.last_frame_time, encoder) ?; core :: result :: Result ::
        Ok(())
    }
}