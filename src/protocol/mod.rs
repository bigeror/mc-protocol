use tokio::{io::AsyncReadExt, net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream}};
use core::net::SocketAddr;

use crate::datatypes::{StringBuffer, VarInt};

#[derive(Debug)]
enum RuntimeError {
    IncorrectProtocol,
    ArcConversionError,
    DecodeError,
    IncorrectIntent,
}

#[derive(PartialEq, Eq)]
enum States {
    HandShake = 0,
    Status = 1,
    Login = 2,
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
                Ok(value) => value,
                Err(error) => { 
                    eprintln!("error decoding packet length, client disconnected"); 
                    return; 
                }
            };

            let mut buffer = vec![0u8; length as usize];
            _ = match this.reader.read_exact(&mut buffer).await {
                Ok(0) => return,
                Ok(n) => n,
                Err(error) => {
                    eprintln!("error getting packet, client disconnected"); 
                    return;                    
                }
            };

            if let Some(err) = handle_packet(&mut this, &buffer) {
                eprintln!("runtime error handling packet: {:?}, client disconnected.", err);
                return;
            };
        }

    })
}

fn handle_packet(this: &mut ProtocolHandler, packet: &Vec<u8>) -> Option<RuntimeError> {
    println!("got new packet: {:?}", packet);
    let protocol = packet[0];

    if this.status == States::HandShake {
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
            0 => this.status = States::Status,
            1 => this.status = States::Login,
            _ => return Some(RuntimeError::IncorrectIntent) 
        }

        return None;
    }

    None
}

