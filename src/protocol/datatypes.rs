use std::{collections::HashMap, sync::Arc};

use tokio::{net::tcp::OwnedReadHalf, sync::mpsc};

use crate::datatypes::{DatatypeError, Packet};

#[derive(Debug)]
pub enum PacketCreateError {
    DatatypeError(DatatypeError),
    AddLengthError,
}
impl From<DatatypeError> for PacketCreateError {
    fn from(err: DatatypeError) -> Self { Self::DatatypeError(err) }
}

#[derive(Debug)]
pub enum RuntimeError {
    IncorrectProtocol,
    ArcConversionError,
    DecodeError(DatatypeError),
    IncorrectIntent,
    PacketCreateError(PacketCreateError),
    UnexpectedNone,
    IncorrectKeepalive,
}
impl From<PacketCreateError> for RuntimeError {
    fn from(error: PacketCreateError) -> Self { Self::PacketCreateError(error) }
}
impl From<DatatypeError> for RuntimeError {
    fn from(error: DatatypeError) -> Self { Self::DecodeError(error) }
}

pub type Responses = HashMap<u8, fn(&mut Packet, &mut ProtocolHandler) -> Option<RuntimeError>>;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum States {
    HandShake,
    Status,
    Login,
    Configuration,
    Play,
}

#[derive(Debug)]
pub struct ProtocolHandler {
    pub status: States,
    pub reader: OwnedReadHalf,
    pub writer: mpsc::UnboundedSender<Vec<u8>>,
    pub protocol_version: i32,
    pub player: Option<Player>,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub username: Arc<str>,
    pub uuid: Arc<str>,
    pub keepalive_num: i64
}

#[derive(Debug)]
pub struct DisconnectInformation {
    pub player: Option<Player>,
    pub disconnect_message: String,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}
impl Vector3<i32> {
    pub fn add(a: Vector3<i32>, b: Vector3<i32>) -> Vector3<i32> {
        Vector3 { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z }
    }
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

pub fn add_length(packet_raw: Result<Vec<u8>, DatatypeError>) -> Result<Vec<u8>, PacketCreateError> {
    let packet = packet_raw?;
    if let Ok(length) = Packet::encode_varint(packet.len() as i32) {
        return Ok(vec![length, packet].concat());
    }
    Err(PacketCreateError::AddLengthError)
}

