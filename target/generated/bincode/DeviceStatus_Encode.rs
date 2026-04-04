impl :: bincode :: Encode for DeviceStatus
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        match self
        {
            Self ::Available
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (0u32), encoder) ?
                ; core :: result :: Result :: Ok(())
            }, Self ::Running
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (1u32), encoder) ?
                ; core :: result :: Result :: Ok(())
            }, Self ::Stopped
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (2u32), encoder) ?
                ; core :: result :: Result :: Ok(())
            },
        }
    }
}