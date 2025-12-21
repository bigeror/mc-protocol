#[macro_export]
macro_rules! concat_buffer {
    (_dev, byte $literal:expr) => {vec![$literal]};
    (_dev, buf $literal:expr) => {$literal};
    (_dev, str $literal:expr) => {crate::datatypes::StringBuffer::encode($literal)?};
    (_dev, varint $literal:expr) => {crate::datatypes::VarInt::encode($literal)?};
    (_dev, int $literal:expr) => {crate::datatypes::Int::encode($literal)};
    (_dev, long $literal:expr) => {crate::datatypes::Long::encode($literal)};
    (_dev, float $literal:expr) => {crate::datatypes::Float::encode($literal)};
    (_dev, double $literal:expr) => {crate::datatypes::Double::encode($literal)};
    (_dev, ushort $literal:expr) => {crate::datatypes::UShort::encode($literal)};
    (_dev, short $literal:expr) => {crate::datatypes::Short::encode($literal)};
    (_dev, uuid $literal:expr) => {crate::datatypes::UUID::encode($literal)?};
    (_dev, pos $literal:expr) => {crate::datatypes::Position::encode($literal)};

    {unwrap: $($type:tt $literal:expr),+ $(,)?} => {
        (|| Ok(concat_buffer!($($type $literal),+)) as Result<Vec<u8>, crate::datatypes::DatatypeError>)().unwrap()
    };
    {$($type:tt $literal:expr),+ $(,)?} => {
        [ $(concat_buffer!(_dev, $type $literal)),+ ].concat()
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

#[macro_export]
macro_rules! try_err {
    ($($item:expr)*) => {
        match $($item)* {
            Ok(val) => val,
            Err(e) => return Some(RuntimeError::from(e))
        }
    };
}

#[macro_export]
macro_rules! try_option_err {
    ($($item:expr)*) => {
        match $($item)* {
            Some(val) => val,
            None => return Some(RuntimeError::UnexpectedNone)
        }
    };
}
