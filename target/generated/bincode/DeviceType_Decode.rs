impl < __Context > :: bincode :: Decode < __Context > for DeviceType
{
    fn decode < __D : :: bincode :: de :: Decoder < Context = __Context > >
    (decoder : & mut __D) ->core :: result :: Result < Self, :: bincode ::
    error :: DecodeError >
    {
        let variant_index = < u32 as :: bincode :: Decode ::< __D :: Context
        >>:: decode(decoder) ?; match variant_index
        {
            0u32 =>core :: result :: Result :: Ok(Self ::Camera {}), 1u32
            =>core :: result :: Result :: Ok(Self ::Microphone {}), variant
            =>core :: result :: Result ::
            Err(:: bincode :: error :: DecodeError :: UnexpectedVariant
            {
                found : variant, type_name : "DeviceType", allowed : &::
                bincode :: error :: AllowedEnumVariants :: Range
                { min: 0, max: 1 }
            })
        }
    }
} impl < '__de, __Context > :: bincode :: BorrowDecode < '__de, __Context >
for DeviceType
{
    fn borrow_decode < __D : :: bincode :: de :: BorrowDecoder < '__de,
    Context = __Context > > (decoder : & mut __D) ->core :: result :: Result <
    Self, :: bincode :: error :: DecodeError >
    {
        let variant_index = < u32 as :: bincode :: Decode ::< __D :: Context
        >>:: decode(decoder) ?; match variant_index
        {
            0u32 =>core :: result :: Result :: Ok(Self ::Camera {}), 1u32
            =>core :: result :: Result :: Ok(Self ::Microphone {}), variant
            =>core :: result :: Result ::
            Err(:: bincode :: error :: DecodeError :: UnexpectedVariant
            {
                found : variant, type_name : "DeviceType", allowed : &::
                bincode :: error :: AllowedEnumVariants :: Range
                { min: 0, max: 1 }
            })
        }
    }
}