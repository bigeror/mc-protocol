#[macro_export]
macro_rules! concat_buffer {
    (_dev, byte $literal:expr) => {Ok(vec![$literal])};
    (_dev, buf $literal:expr) => {Ok($literal)};
    (_dev, str $literal:expr) => {crate::datatypes::Packet::encode_string($literal)};
    (_dev, varint $literal:expr) => {crate::datatypes::Packet::encode_varint($literal)};
    (_dev, int $literal:expr) => {Ok(crate::datatypes::Packet::encode_int($literal))};
    (_dev, long $literal:expr) => {Ok(crate::datatypes::Packet::encode_long($literal))};
    (_dev, float $literal:expr) => {Ok(crate::datatypes::Packet::encode_float($literal))};
    (_dev, double $literal:expr) => {Ok(crate::datatypes::Packet::encode_double($literal))};
    (_dev, ushort $literal:expr) => {Ok(crate::datatypes::Packet::encode_ushort($literal))};
    (_dev, short $literal:expr) => {Ok(crate::datatypes::Packet::encode_short($literal))};
    (_dev, uuid $literal:expr) => {crate::datatypes::Packet::encode_uuid($literal)};
    (_dev, pos $literal:expr) => {Ok(crate::datatypes::Packet::encode_position($literal))};

    {$($type:tt $literal:expr),+ $(,)?} => {
        (
        [ $(concat_buffer!(_dev, $type $literal)),+ ]
            .into_iter()
            .collect::<Result<Vec<Vec<u8>>, crate::datatypes::DatatypeError>>()
        )
    };
}

#[macro_export]
macro_rules! create_packet_collection {
    ($name:tt, $($field:tt : |$($arg:ident : $type:ty),*| $code:block),+ $(,)?) => {
        pub struct $name {
            $(pub $field : fn($($arg : $type),*) -> Result<Vec<u8>, crate::protocol::datatypes::PacketCreateError>),+ 
        }
        impl $name {
            pub fn init() -> $name {
                $name {$($field : |$($arg),*| {crate::protocol::datatypes::add_length($code)}),+}
            }
        }
    };
}
