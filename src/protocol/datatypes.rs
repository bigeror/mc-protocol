use std::{collections::HashMap, ops::Add, sync::Arc};

use tokio::{net::tcp::OwnedReadHalf, sync::mpsc};

use crate::datatypes::{DatatypeError, Packet};

#[derive(Debug)]
pub enum PacketCreateError {
    DatatypeError(DatatypeError),
    AddLengthError,
    ArcAsMutError,
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
    IncorrectValue,
}
impl From<PacketCreateError> for RuntimeError {
    fn from(error: PacketCreateError) -> Self { Self::PacketCreateError(error) }
}
impl From<DatatypeError> for RuntimeError {
    fn from(error: DatatypeError) -> Self { Self::DecodeError(error) }
}

pub type Responses = HashMap<u8, fn(&mut Packet, &mut ProtocolHandler) -> Result<(), RuntimeError>>;

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
    pub eid: i32,
    pub username: Arc<str>,
    pub uuid: u128,
    pub keepalive_num: i64,
    pub hotbar: [i32; 9],
    pub selected_slot: i16,
    pub position: Vector3<f64>,
    pub rotation: Vector2<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerKey {
    pub uuid: u128,
    pub username: Arc<str>,
    pub eid: i32,
}

impl From<Player> for PlayerKey {
    fn from(val: Player) -> Self {
        Self { uuid: val.uuid, username: val.username, eid: val.eid }
    }
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
impl<T: Add<Output = T>> Add for Vector3<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Vector3 { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
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

pub trait Display where Self: Sized {
    fn display(&self) -> String;
    fn from_string(string: &str) -> Result<Self, DatatypeError>;
}

impl Display for u128 {
    fn display(&self) -> String {
        self.to_be_bytes()
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<String>>()
            .join("")
    }
    fn from_string(string: &str) -> Result<Self, DatatypeError> {
        match u128::from_str_radix(string, 16) {
            Ok(val) => Ok(val),
            Err(err) => Err(DatatypeError::ParseIntError(err))
        }
    }
}

