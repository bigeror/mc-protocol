use core::str;
use std::{io::ErrorKind, num::ParseIntError, str::Utf8Error, sync::Arc};

use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf, sync::Mutex};

use crate::protocol::datatypes::Vector3;

#[derive(Debug)]
pub enum DatatypeError {
    NotExistingCode,
    TooSmallBuffer,
    VarIntTooBig,
    StreamError,
    Utf8DecodeError(Utf8Error),
    ParseIntError(ParseIntError),
    ParseError,
}

#[derive(Debug)]
pub struct Packet {
    pub source: Arc<[u8]>,
    pub offset: usize,
}

// on reading end it returns Ok(0), which is impossible to get otherwise in packet length.
pub async fn decode_packet_length(reader: &mut OwnedReadHalf, decryptor: Option<Arc<Mutex<cfb8::Decryptor<aes::Aes128>>>>) -> Result<i32, DatatypeError> {
    let mut position: u32 = 0;
    let mut result: i32 = 0;

    loop {
        let current_byte = match reader.read_u8().await {
            Ok(value) => if let Some(decryptor) = decryptor.clone() {
                let mut encrypted_value = [value];
                decryptor.lock().await.decrypt(&mut encrypted_value);
                encrypted_value[0]
            }
            else { value },
            Err(error) => {
                if error.kind() == ErrorKind::UnexpectedEof { return Ok(0) }
                else { return Err(DatatypeError::StreamError) }
            }
        };
        result |= ((current_byte & 0x7F) as i32) << position;
        if (current_byte & 0x80) == 0 { break };
        position += 7;
        if position >= 32 { return Err(DatatypeError::VarIntTooBig) }
    }

    Ok(result)
}

impl Packet {
    pub fn new(packet: Vec<u8>) -> Self {
        Self { source: packet.into(), offset: 0 }
    }

    pub fn read_u8(&mut self) -> Result<u8, DatatypeError> {
        self.offset += 1;
        if self.offset > self.source.len() { return Err(DatatypeError::TooSmallBuffer) }
        Ok(self.source[self.offset - 1])
    }
    pub fn read_buf(&mut self, amount: usize) -> Result<Vec<u8>, DatatypeError> {
        if self.offset + amount > self.source.len() { return Err(DatatypeError::TooSmallBuffer) }
        let output = self.source[self.offset..self.offset + amount].to_vec();
        self.offset += amount;
        Ok(output)
    }

    pub fn take_slice<const AMOUNT: usize> (&mut self) -> Result<[u8; AMOUNT], DatatypeError> {
        if self.offset + AMOUNT > self.source.len() {return Err(DatatypeError::TooSmallBuffer)} // bounds checking
        let initial_offset = self.offset;
        self.offset += AMOUNT;
        Ok(unsafe {*(self.source.as_ptr().add(initial_offset) as *const [u8; AMOUNT])})
    }

    pub fn decode_varint(&mut self) -> Result<i32, DatatypeError> {
        let mut position: u32 = 0;
        let mut result: i32 = 0;
        let max_offset = self.source.len();

        loop {
            let current_byte = self.read_u8()?;
            result = result | (((current_byte & 0x7F) as i32) << position);
            if (current_byte & 0x80) == 0 { break };
            position += 7;
            if position >= 32 { return Err(DatatypeError::VarIntTooBig) }
        }

        Ok(result)
    }
    pub fn decode_string(&mut self) -> Result<String, DatatypeError> {
        let length = self.decode_varint()?;
        let buffer = self.read_buf(length as usize)?;
        let string = match str::from_utf8(&buffer) {
            Ok(str) => str.to_string(),
            Err(e) => return Err(DatatypeError::Utf8DecodeError(e)),
        };
        Ok(string)
    }
    pub fn decode_position(&mut self) -> Result<Vector3<i32>, DatatypeError> {
        let long = self.decode_long()?;
        Ok(Vector3 { 
            x: (long >> 38) as i32,
            y: (long << 52 >> 52) as i32,
            z: (long << 26 >> 38) as i32
        })
    }
    pub fn decode_int(&mut self) -> Result<i32, DatatypeError> { Ok(i32::from_be_bytes(self.take_slice::<4>()?)) }
    pub fn decode_long(&mut self) -> Result<i64, DatatypeError> { Ok(i64::from_be_bytes(self.take_slice::<8>()?)) }
    pub fn decode_float(&mut self) -> Result<f32, DatatypeError> { Ok(f32::from_be_bytes(self.take_slice::<4>()?)) }
    pub fn decode_double(&mut self) -> Result<f64, DatatypeError> { Ok(f64::from_be_bytes(self.take_slice::<8>()?)) }
    pub fn decode_short(&mut self) -> Result<i16, DatatypeError> { Ok(i16::from_be_bytes(self.take_slice::<2>()?)) }
    pub fn decode_ushort(&mut self) -> Result<u16, DatatypeError> { Ok(u16::from_be_bytes(self.take_slice::<2>()?)) }
    pub fn decode_uuid(&mut self) -> Result<u128, DatatypeError> { Ok(u128::from_be_bytes(self.take_slice::<16>()?)) }

    pub fn encode_varint(input: i32) -> Result<Vec<u8>, DatatypeError> {
        let mut value = input.clone();
        let mut output: Vec<u8> = Vec::new();

        while (value & !(127 as i32)) != 0 {
            output.push(((value & 127) + 128) as u8);
            value = value / 128
        }
        output.push(value as u8);

        Ok(output)
    }
    pub fn encode_string(input: &str) -> Result<Vec<u8>, DatatypeError> {
        let array = input.as_bytes().to_vec();
        let mut result = Packet::encode_varint(array.len() as i32)?;
        result.extend(array.iter());
        Ok(result)
    }
    pub fn encode_position(input: Vector3<i32>) -> Vec<u8> {
        Packet::encode_long(((input.x as i64 & 0x3FFFFFF) << 38)
            | ((input.z as i64 & 0x3FFFFFF) << 12)
            | (input.y as i64 & 0xFFF))
    }
    pub fn encode_int(input: i32) -> Vec<u8> { input.to_be_bytes().to_vec() }
    pub fn encode_long(input: i64) -> Vec<u8> { input.to_be_bytes().to_vec() }
    pub fn encode_float(input: f32) -> Vec<u8> { input.to_be_bytes().to_vec() }
    pub fn encode_double(input: f64) -> Vec<u8> { input.to_be_bytes().to_vec() }
    pub fn encode_short(input: i16) -> Vec<u8> { input.to_be_bytes().to_vec() }
    pub fn encode_ushort(input: u16) -> Vec<u8> { input.to_be_bytes().to_vec() }
    pub fn encode_uuid(input: u128) -> Vec<u8> { input.to_be_bytes().to_vec() }
}
