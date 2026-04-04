impl :: bincode :: Encode for DeviceInformation
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.id, encoder) ?; :: bincode ::
        Encode :: encode(&self.name, encoder) ?; :: bincode :: Encode ::
        encode(&self.device_type, encoder) ?; :: bincode :: Encode ::
        encode(&self.device_status, encoder) ?; core :: result :: Result ::
        Ok(())
    }
}