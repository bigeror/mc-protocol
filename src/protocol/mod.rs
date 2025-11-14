mod initialisation;

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}}};
use core::net::SocketAddr;

use crate::{datatypes::{StringBuffer, VarInt}, protocol::initialisation::serverbound::SERVER_BOUND_PACKETS_INSTANCE};

#[derive(Debug)]
enum RuntimeError {
    IncorrectProtocol,
    ArcConversionError,
    DecodeError,
    IncorrectIntent,
}

#[derive(PartialEq, Eq)]
enum States {
    HandShake,
    Status,
    Login,
    Configuration,
    Play,
}

struct ProtocolHandler {
    status: States,
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    protocol_version: u64
}


pub fn protocol_handler_main(client: TcpStream, address: SocketAddr) {
    _ = tokio::spawn(async move {

        let (reader, writer) = client.into_split();
        let mut this = ProtocolHandler {
            status: States::HandShake,
            reader, writer,
            protocol_version: 0
        };

        loop {
            let length = match VarInt::decode_packet_length(&mut this.reader).await {
                Ok(value) => if value == 0 {
                    _ = this.writer.shutdown().await;
                    return;
                } else { value },
                Err(error) => {
                    eprintln!("error decoding packet length, client disconnected"); 
                    _ = this.writer.shutdown().await;
                    return;
                }
            };

            let mut buffer = vec![0u8; length as usize];
            _ = match this.reader.read_exact(&mut buffer).await {
                Ok(0) => return,
                Ok(n) => if n <= (2 ^ 20) {n} else {
                    eprintln!("packet too large, client disconnected"); 
                    _ = this.writer.shutdown().await;
                    return;
                } ,
                Err(error) => {
                    eprintln!("error getting packet, client disconnected"); 
                    _ = this.writer.shutdown().await;
                    return;
                }
            };

            if let Some(err) = handle_packet(&mut this, &buffer).await {
                eprintln!("runtime error handling packet: {:?}, client disconnected.", err);
                _ = this.writer.shutdown().await;
                return;
            };
        }

    })
}

async fn handle_packet(this: &mut ProtocolHandler, packet: &Vec<u8>) -> Option<RuntimeError> {
    println!("got new packet: {:?}", packet);

    let mut error: Option<RuntimeError> = None;
    let protocol = packet[0];
    if this.status == States::HandShake { return handle_handshake(this, packet, protocol); }
    else if this.status == States::Status {
        error = match SERVER_BOUND_PACKETS_INSTANCE.status.get(&protocol) {
        Some(func) => func(packet, this).await,
        None => None
    } }

    error
}

fn handle_handshake(this: &mut ProtocolHandler, packet: &Vec<u8>, protocol: u8) -> Option<RuntimeError> {
    if !protocol == 0 { return Some(RuntimeError::IncorrectProtocol) };

    let protocol_version_raw = match VarInt(&packet).decode(1) {
        Ok(value) => value,
        Err(error) => return Some(RuntimeError::DecodeError)
    };
    this.protocol_version = protocol_version_raw.value;

    let mut offset = match StringBuffer(&packet).decode(protocol_version_raw.offset) {
        Ok(value) => value.offset,
        Err(error) => return Some(RuntimeError::DecodeError)
    };
    offset += 2;

    let intent = match VarInt(&packet).decode(offset) {
        Ok(value) => value.value,
        Err(error) => return Some(RuntimeError::DecodeError)
    };

    match intent {
        1 => this.status = States::Status,
        2 => this.status = States::Login,
        _ => return Some(RuntimeError::IncorrectIntent) 
    };

    None
}