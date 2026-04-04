impl < __Context > :: bincode :: Decode < __Context > for
FramePacketInformation
{
    fn decode < __D : :: bincode :: de :: Decoder < Context = __Context > >
    (decoder : & mut __D) ->core :: result :: Result < Self, :: bincode ::
    error :: DecodeError >
    {
        core :: result :: Result ::
        Ok(Self
        {
            device_info : :: bincode :: Decode :: decode(decoder) ?,
            rx_timestamp : :: bincode :: Decode :: decode(decoder) ?,
            tx_timestamp : :: bincode :: Decode :: decode(decoder) ?,
            frame_shape : :: bincode :: Decode :: decode(decoder) ?,
        })
    }
} impl < '__de, __Context > :: bincode :: BorrowDecode < '__de, __Context >
for FramePacketInformation
{
    fn borrow_decode < __D : :: bincode :: de :: BorrowDecoder < '__de,
    Context = __Context > > (decoder : & mut __D) ->core :: result :: Result <
    Self, :: bincode :: error :: DecodeError >
    {
        core :: result :: Result ::
        Ok(Self
        {
            device_info : :: bincode :: BorrowDecode ::< '_, __Context >::
            borrow_decode(decoder) ?, rx_timestamp : :: bincode ::
            BorrowDecode ::< '_, __Context >:: borrow_decode(decoder) ?,
            tx_timestamp : :: bincode :: BorrowDecode ::< '_, __Context >::
            borrow_decode(decoder) ?, frame_shape : :: bincode :: BorrowDecode
            ::< '_, __Context >:: borrow_decode(decoder) ?,
        })
    }
}