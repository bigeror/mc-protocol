use core::str;
use std::{io::ErrorKind, num::{self, ParseIntError}, str::Utf8Error};

use regex::Regex;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};

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
pub struct DecodeResult<T> {
    pub value: T,
    pub offset: u32,
}

pub struct VarInt<'a>(pub &'a Vec<u8>);
impl<'a> VarInt<'a> {
    pub fn encode(input: i32) -> Result<Vec<u8>, DatatypeError> {
        let mut value = input.clone();
        let mut output: Vec<u8> = Vec::new();

        while (value & !(127 as i32)) != 0 {
            output.push(((value & 127) + 128) as u8);
            value = value / 128
        }
        output.push(value as u8);

        Ok(output)
    }

    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<i32>, DatatypeError> {
        let mut offset: u32 = initial_offset.clone();
        let mut position: u32 = 0;
        let mut result: i32 = 0;
        let max_length = self.0.len() as u32;

        loop {
            if offset >= max_length {
                return Err(DatatypeError::TooSmallBuffer);
            };
            let current_byte = self.0[offset as usize];
            offset += 1;
            result = result | (((current_byte & 0x7F) as i32) << position);
            if (current_byte & 0x80) == 0 {
                break;
            };
            position += 7;
            if position >= 32 {
                return Err(DatatypeError::VarIntTooBig);
            }
        }

        Ok(DecodeResult {
            value: result,
            offset,
        })
    }

    // on reading end it returns Ok(0), which is impossible to get otherwise in packet length.
    pub async fn decode_packet_length(reader: &mut OwnedReadHalf) -> Result<i32, DatatypeError> {
        let mut position: u32 = 0;
        let mut result: i32 = 0;

        loop {
            let current_byte = match reader.read_u8().await {
                Ok(value) => value,
                Err(error) => {
                    if error.kind() == ErrorKind::UnexpectedEof {
                        return Ok(0);
                    } else {
                        return Err(DatatypeError::StreamError);
                    }
                }
            };
            result |= ((current_byte & 0x7F) as i32) << position;
            if (current_byte & 0x80) == 0 {
                break;
            };
            position += 7;
            if position >= 32 {
                return Err(DatatypeError::VarIntTooBig);
            }
        }

        Ok(result)
    }
}

pub struct StringBuffer<'a>(pub &'a Vec<u8>);
impl<'a> StringBuffer<'a> {
    pub fn encode(input: &str) -> Result<Vec<u8>, DatatypeError> {
        let array = input.as_bytes().to_vec();
        let mut result = VarInt::encode(array.len() as i32)?;
        result.extend(array.iter());
        Ok(result)
    }

    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<String>, DatatypeError> {
        let length = VarInt::decode(&VarInt(self.0), initial_offset)?;
        if length.offset + length.value as u32 >= self.0.len() as u32 {
            return Err(DatatypeError::TooSmallBuffer);
        }
        let buffer =
            self.0[length.offset as usize..(length.offset + length.value as u32) as usize].to_vec();
        let string = match str::from_utf8(&buffer) {
            Ok(str) => str.to_string(),
            Err(e) => return Err(DatatypeError::Utf8DecodeError(e)),
        };
        Ok(DecodeResult {
            value: string,
            offset: length.offset + length.value as u32,
        })
    }
}

#[derive(Debug)]
pub struct UUID<'a>(pub &'a Vec<u8>);
impl<'a> UUID<'a> {
    pub fn encode(input: &str) -> Result<Vec<u8>, DatatypeError> {
        let re = Regex::new(r"(..?)").unwrap();
        let output: Result<Vec<u8>, num::ParseIntError> = re.captures_iter(input.replace("-", "").as_str())
            .map(|caps| u8::from_str_radix(&caps[0], 16))
            .collect();
        match output {
            Ok(val) => Ok(val),
            Err(e) => Err(DatatypeError::ParseIntError(e))
        }
    }
    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<String>, DatatypeError> {
        if initial_offset + 16 > self.0.len() as u32 {
            return Err(DatatypeError::TooSmallBuffer);
        }
        let string: String = self.0[initial_offset as usize..initial_offset as usize + 16]
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<String>>()
            .join("");

        Ok(DecodeResult {
            value: string,
            offset: initial_offset + 16,
        })
    }
}

#[derive(Debug)]
pub struct Int<'a>(pub &'a Vec<u8>);
impl<'a> Int<'a> {
    pub fn encode(input: i32) -> Vec<u8> {
        input.to_be_bytes().to_vec()
    }
    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<i32>, DatatypeError> {
        if initial_offset + 4 > self.0.len() as u32 {
            return Err(DatatypeError::TooSmallBuffer)
        }
        let mut bytes: [u8; 4] = [0; 4];
        bytes.copy_from_slice(&self.0[initial_offset as usize..initial_offset as usize + 4]);
        Ok(DecodeResult {
            offset: initial_offset + 4,
            value: i32::from_be_bytes(bytes)
        })
    }
}

#[derive(Debug)]
pub struct Long<'a>(pub &'a Vec<u8>);
impl<'a> Long<'a> {
    pub fn encode(input: i64) -> Vec<u8> {
        input.to_be_bytes().to_vec()
    }
    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<i64>, DatatypeError> {
        if initial_offset + 8 > self.0.len() as u32 {
            return Err(DatatypeError::TooSmallBuffer)
        }
        let mut bytes: [u8; 8] = [0; 8];
        bytes.copy_from_slice(&self.0[initial_offset as usize..initial_offset as usize + 8]);
        Ok(DecodeResult {
            offset: initial_offset + 8,
            value: i64::from_be_bytes(bytes)
        })
    }
}

#[derive(Debug)]
pub struct Float<'a>(pub &'a Vec<u8>);
impl<'a> Float<'a> {
    pub fn encode(input: f32) -> Vec<u8> {
        input.to_be_bytes().to_vec()
    }
    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<f32>, DatatypeError> {
        if initial_offset + 4 > self.0.len() as u32 {
            return Err(DatatypeError::TooSmallBuffer)
        }
        let mut bytes: [u8; 4] = [0; 4];
        bytes.copy_from_slice(&self.0[initial_offset as usize..initial_offset as usize + 4]);
        Ok(DecodeResult {
            offset: initial_offset + 4,
            value: f32::from_be_bytes(bytes)
        })
    }
}

#[derive(Debug)]
pub struct Double<'a>(pub &'a Vec<u8>);
impl<'a> Double<'a> {
    pub fn encode(input: f64) -> Vec<u8> {
        input.to_be_bytes().to_vec()
    }
    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<f64>, DatatypeError> {
        if initial_offset + 8 > self.0.len() as u32 {
            return Err(DatatypeError::TooSmallBuffer)
        }
        let mut bytes: [u8; 8] = [0; 8];
        bytes.copy_from_slice(&self.0[initial_offset as usize..initial_offset as usize + 8]);
        Ok(DecodeResult {
            offset: initial_offset + 8,
            value: f64::from_be_bytes(bytes)
        })
    }
}

#[derive(Debug)]
pub struct UShort<'a>(pub &'a Vec<u8>);
impl<'a> UShort<'a> {
    pub fn encode(input: u16) -> Vec<u8> {
        input.to_be_bytes().to_vec()
    }
    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<u16>, DatatypeError> {
        if initial_offset + 2 > self.0.len() as u32 {
            return Err(DatatypeError::TooSmallBuffer)
        }
        let mut bytes: [u8; 2] = [0; 2];
        bytes.copy_from_slice(&self.0[initial_offset as usize..initial_offset as usize + 2]);
        Ok(DecodeResult {
            offset: initial_offset + 2,
            value: u16::from_be_bytes(bytes)
        })
    }
}

#[derive(Debug)]
pub struct Short<'a>(pub &'a Vec<u8>);
impl<'a> Short<'a> {
    pub fn encode(input: i16) -> Vec<u8> {
        input.to_be_bytes().to_vec()
    }
    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<i16>, DatatypeError> {
        if initial_offset + 2 > self.0.len() as u32 {
            return Err(DatatypeError::TooSmallBuffer)
        }
        let mut bytes: [u8; 2] = [0; 2];
        bytes.copy_from_slice(&self.0[initial_offset as usize..initial_offset as usize + 2]);
        Ok(DecodeResult {
            offset: initial_offset + 2,
            value: i16::from_be_bytes(bytes)
        })
    }
}

#[derive(Debug)]
pub struct Position<'a>(pub &'a Vec<u8>);
impl<'a> Position<'a> {
    pub fn encode(input: Vector3<i32>) -> Vec<u8> {
        let long: i64 = ((input.x as i64 & 0x3FFFFFF) << 38)
            | ((input.z as i64 & 0x3FFFFFF) << 12)
            | (input.y as i64 & 0xFFF);
        Long::encode(long)
    }
    pub fn decode(&self, initial_offset: u32) -> Result<DecodeResult<Vector3<i32>>, DatatypeError> {
        let long = Long(self.0).decode(initial_offset)?.value;
        Ok(DecodeResult {
            offset: initial_offset + 8,
            value: Vector3 { 
                x: (long >> 38) as i32,
                y: (long << 52 >> 52) as i32,
                z: (long << 26 >> 38) as i32
            }
        })
    }
}

