use core::str;
use std::io::ErrorKind;

use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};

#[derive(Debug)]
pub enum DatatypeError {
    TooSmallBuffer,
    VarIntTooBig,
    StreamError,
    Utf8DecodeError,
}

pub struct DecodeResult<T> {
    pub value: T,
    pub offset: u32
}

pub struct VarInt<'a>(pub &'a Vec<u8>);
impl<'a> VarInt<'a> {
    pub fn encode(input: u64) -> Result<Vec<u8>, DatatypeError> {
        let mut value = input.clone();
        let mut output: Vec<u8> = Vec::new();

        while (value & !0x07) != 0 {
            output.push(((value & 0x7F) | 0x80) as u8);
            value = value >> 7
        }

        Ok(output)
    }

    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<u64>, DatatypeError> {
        let mut offset: u32 = initial_offset.clone();
        let mut position: u64 = 0;
        let mut result: u64 = 0;
        let max_length = self.0.len() as u32;

        loop {
            if offset >= max_length { return Err(DatatypeError::TooSmallBuffer) };
            let current_byte = self.0[offset as usize];
            offset += 1;
            result = result | (((current_byte & 0x7F) as u64) << position);
            if (current_byte & 0x80) == 0 { break };
            position += 7;
            if position >= 32 { return Err(DatatypeError::VarIntTooBig) }
        }

        Ok(DecodeResult { value: result, offset })
    }

    // on reading end it returns Ok(0), which is impossible to get otherwise in packet length.
    pub async fn decode_packet_length(reader: &mut OwnedReadHalf) -> Result<u64, DatatypeError> { 
        let mut position: u64 = 0;
        let mut result: u64 = 0;

        loop {
            let current_byte = match reader.read_u8().await {
                Ok(value) => value,
                Err(error) => if error.kind() == ErrorKind::UnexpectedEof { return Ok(0) } else {return Err(DatatypeError::StreamError)}
            };
            result = result | (((current_byte & 0x7F) as u64) << position);
            if (current_byte & 0x80) == 0 { break };
            position += 7;
            if position >= 32 { return Err(DatatypeError::VarIntTooBig) }
        }

        Ok(result)
    }
}

pub struct StringBuffer<'a>(pub &'a Vec<u8>);
impl<'a> StringBuffer<'a> {
    pub fn encode(input: &str) -> Result<Vec<u8>, DatatypeError> {
        let array = input.as_bytes().to_vec();
        let mut result = VarInt::encode(array.len() as u64)?;
        result.extend(array.iter());
        Ok(result)
    }

    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<String>, DatatypeError> {
        let length = VarInt::decode(&VarInt(&self.0), initial_offset)?;
        let buffer = self.0[length.offset as usize..(length.offset + length.value as u32) as usize].to_vec();
        let string = match str::from_utf8(&buffer) {
            Ok(str) => str.to_string(),
            Err(e) => return Err(DatatypeError::Utf8DecodeError)
        };
        Ok(DecodeResult { value: string, offset: length.offset + length.value as u32 })
    }
}