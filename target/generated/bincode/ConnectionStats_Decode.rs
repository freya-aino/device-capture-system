impl < __Context > :: bincode :: Decode < __Context > for ConnectionStats
{
    fn decode < __D : :: bincode :: de :: Decoder < Context = __Context > >
    (decoder : & mut __D) ->core :: result :: Result < Self, :: bincode ::
    error :: DecodeError >
    {
        core :: result :: Result ::
        Ok(Self
        {
            frames : :: bincode :: Decode :: decode(decoder) ?, bytes : ::
            bincode :: Decode :: decode(decoder) ?, current_fps : :: bincode
            :: Decode :: decode(decoder) ?, current_bitrate : :: bincode ::
            Decode :: decode(decoder) ?, current_latency : :: bincode ::
            Decode :: decode(decoder) ?, start_time : :: bincode :: Decode ::
            decode(decoder) ?, last_frame_time : :: bincode :: Decode ::
            decode(decoder) ?,
        })
    }
} impl < '__de, __Context > :: bincode :: BorrowDecode < '__de, __Context >
for ConnectionStats
{
    fn borrow_decode < __D : :: bincode :: de :: BorrowDecoder < '__de,
    Context = __Context > > (decoder : & mut __D) ->core :: result :: Result <
    Self, :: bincode :: error :: DecodeError >
    {
        core :: result :: Result ::
        Ok(Self
        {
            frames : :: bincode :: BorrowDecode ::< '_, __Context >::
            borrow_decode(decoder) ?, bytes : :: bincode :: BorrowDecode ::<
            '_, __Context >:: borrow_decode(decoder) ?, current_fps : ::
            bincode :: BorrowDecode ::< '_, __Context >::
            borrow_decode(decoder) ?, current_bitrate : :: bincode ::
            BorrowDecode ::< '_, __Context >:: borrow_decode(decoder) ?,
            current_latency : :: bincode :: BorrowDecode ::< '_, __Context >::
            borrow_decode(decoder) ?, start_time : :: bincode :: BorrowDecode
            ::< '_, __Context >:: borrow_decode(decoder) ?, last_frame_time :
            :: bincode :: BorrowDecode ::< '_, __Context >::
            borrow_decode(decoder) ?,
        })
    }
}