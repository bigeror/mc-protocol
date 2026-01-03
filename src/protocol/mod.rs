mod initialisation;
mod play;
mod macros;
pub mod datatypes;
pub mod server;

use core::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use crab_nbt::nbt;
use futures::FutureExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, 
    net::{TcpStream, tcp::OwnedWriteHalf}, sync::mpsc,
};

use crate::{
    datatypes::{Packet, decode_packet_length},
    protocol::{
        datatypes::{
            Player, ProtocolHandler, RuntimeError, States
        }, initialisation::serverbound::SERVER_BOUND_PACKETS_INSTANCE as SERVER_BOUND_PACKETS_INSTANCE_INIT, play::{clientbound::CLIENT_BOUND_PACKETS, serverbound::SERVERBOUND_PACKET_INSTANCE}, server::server::SERVER
    },
};

pub fn protocol_handler_main(client: TcpStream, address: SocketAddr) {
    _ = tokio::spawn(async move {
        let (reader, writer) = client.into_split();
        let mut this = ProtocolHandler {
            status: States::HandShake,
            reader,
            writer: setup_writer(writer).await,
            protocol_version: 0,
            player: None,
        };

        let result = AssertUnwindSafe(protocol_handler(address, &mut this))
            .catch_unwind().await;

        match result {
            Ok(val) => (),
            Err(err) => {
                if let Some(error) = err.downcast_ref::<&str>() { println!("got fatal error: {}", error) } 
                else { println!("got fatal error without message") }
            },
        };

        match (this.player.clone(), this.status == States::Configuration || this.status == States::Play) {
            (Some(player), true) => {
                let mut server = SERVER.lock().await;
                server.players.remove(&(player.clone().uuid, player.clone().username));

                let message = format!("{} left the game.", player.username);
                server.send_to_players((CLIENT_BOUND_PACKETS.send_system_message)(nbt!("", {
                    "text": message,
                    "color": "yellow",
                }).write_unnamed().to_vec(), false).unwrap(), None);
                println!("Player {} [{}] disconnected the game.", player.username, player.uuid);
            },
            (_, true) => panic!("Got unexpected state: handler is in configuration / play but no player information."),
            _ => ()
        };
        _ = this.writer.send([0].into());
        return;
    })
}

async fn setup_writer(mut writer: OwnedWriteHalf) -> mpsc::UnboundedSender<Vec<u8>> {
    let (sender, mut reader) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(async move {
        loop {
            if let Some(mut message) = reader.recv().await {
                if message == vec![0] {
                    _ = writer.shutdown();
                    reader.close();
                    return;
                }
                _ = writer.write_all(&mut message).await;
            }
        }
    });
    sender
}

async fn protocol_handler(address: SocketAddr, handler: &mut ProtocolHandler) {
    loop {
        let length = match decode_packet_length(&mut handler.reader).await {
            Ok(value) => {
                if value == 0 { return } 
                else { value }
            }
            Err(error) => {
                eprintln!("error decoding packet length, client disconnected");
                return;
            }
        };

        let mut buffer = vec![0u8; length as usize];
        _ = match handler.reader.read_exact(&mut buffer).await {
            Ok(0) => return,
            Ok(n) => {
                if n <= (2 << 20) { n } else {
                    eprintln!("packet too large, client disconnected");
                    return;
                }
            }
            Err(error) => {
                eprintln!("error getting packet, client disconnected");
                return;
            }
        };

        let mut packet = Packet::new(buffer);
        if let Some(err) = handle_packet(handler, &mut packet).await {
            eprintln!("runtime error handling packet: {:?}, client disconnected.", err);
            return;
        };
    }
}

async fn handle_packet(this: &mut ProtocolHandler, packet: &mut Packet) -> Option<RuntimeError> {
    let mut error: Option<RuntimeError> = None;
    let protocol = match packet.read_u8() {
        Ok(value) => value,
        Err(err) => return Some(err.into())
    };
    if this.status == States::HandShake {
        return handle_handshake(this, packet, protocol);
    } else if this.status == States::Status {
        error = match SERVER_BOUND_PACKETS_INSTANCE_INIT.status.get(&protocol) {
            Some(func) => func(packet, this),
            None => Some(RuntimeError::IncorrectProtocol),
        }
    } else if this.status == States::Login {
        error = match SERVER_BOUND_PACKETS_INSTANCE_INIT.login.get(&protocol) {
            Some(func) => func(packet, this),
            None => None,
        }
    } else if this.status == States::Configuration {
        error = match SERVER_BOUND_PACKETS_INSTANCE_INIT.configuration.get(&protocol) {
            Some(func) => func(packet, this),
            None => None,
        }
    } else if this.status == States::Play {
        error = match  SERVERBOUND_PACKET_INSTANCE.get(&protocol) {
            Some(func) => func(packet, this),
            None => None,
        }
    }

    error
}

fn handle_handshake( this: &mut ProtocolHandler, packet: &mut Packet, protocol: u8 ) -> Option<RuntimeError> {
    if !protocol == 0 { return Some(RuntimeError::IncorrectProtocol) };

    let protocol_version = match packet.decode_varint() {
        Ok(value) => value,
        Err(error) => return Some(RuntimeError::DecodeError(error)),
    };

    let used_ip = match packet.decode_string() {
        Ok(value) => value,
        Err(error) => return Some(RuntimeError::DecodeError(error)),
    };
    _ = packet.decode_ushort();

    let intent = match packet.decode_varint() {
        Ok(value) => value,
        Err(error) => return Some(RuntimeError::DecodeError(error)),
    };

    match intent {
        1 => this.status = States::Status,
        2 => this.status = States::Login,
        _ => return Some(RuntimeError::IncorrectIntent), // there exist intent 3 but it's ignored
    };

    None
}

