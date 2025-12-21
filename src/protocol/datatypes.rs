use std::{collections::HashMap, sync::Arc};

use tokio::{net::tcp::OwnedReadHalf, sync::{Mutex, mpsc}};

use crate::{datatypes::{DatatypeError, VarInt}, protocol::server::player_game::Game};

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

pub type Responses = HashMap<u8, fn(&Vec<u8>, &mut ProtocolHandler) -> Option<RuntimeError>>;

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
    pub game: Option<Arc<Mutex<Game>>>,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub username: Arc<str>,
    pub uuid: Arc<str>,
    pub keepalive_num: i64,
    pub rotation: Vector2<f32>,
}

#[derive(Debug)]
pub struct DisconnectInformation {
    pub player: Option<Player>,
    pub disconnect_message: String,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
}
impl PlayerInput {
    pub fn empty() -> Self {Self {
        forward: false,
        backward: false,
        left: false,
        right: false,
        jump: false,
        sneak: false,
        sprint: false,
    }}
    pub fn from_flags(flags: u8) -> Self {Self {
        forward: (flags & 0x01) > 0,
        backward: (flags & 0x02) > 0,
        left: (flags & 0x04) > 0,
        right: (flags & 0x08) > 0,
        jump: (flags & 0x10) > 0,
        sneak: (flags & 0x20) > 0,
        sprint: (flags & 0x40) > 0,
    }}
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
    if let Ok(length) = VarInt::encode(packet.len() as i32) {
        return Ok(vec![length, packet].concat());
    }
    Err(PacketCreateError::AddLengthError)
}

