mod initialisation;
mod play;
mod macros;
pub mod datatypes;
pub mod server;
pub mod cryptography;

use core::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use futures::FutureExt;
use rsa::errors;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, tcp::OwnedWriteHalf}, sync::mpsc,
};

use crate::{
    datatypes::{Packet, decode_packet_length},
    protocol::{
        datatypes::{ Player, PlayerKey, ProtocolHandler, RuntimeError, States }, initialisation::serverbound::{configuration_response, login_response, status_response}, play::serverbound::{SERVERBOUND_PACKET_INSTANCE, play_responses}, server::server::{Data, SERVER}
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

        let result = AssertUnwindSafe(
            protocol_handler(address, &mut this) // Catch user specific exceptions so server doesn't crash
        ).catch_unwind().await;

        match result {
            Ok(val) => (),
            Err(err) => {
                if let Some(error) = err.downcast_ref::<&str>() { println!("got fatal error: {}", error) }
                else { println!("got fatal error without message") }
            },
        };

        match (this.player.clone(), this.status == States::Configuration || this.status == States::Play) {
            (Some(player), true) => {
                _ = SERVER.sender.send(Data::RemovePlayer { player: PlayerKey::from(&player) });
            },
            (_, true) => panic!("Got unexpected state: handler is in configuration / play but no player information."),
            _ => ()
        };
        _ = this.writer.send([0].into());
        return;
    })
}

async fn setup_writer(mut writer: OwnedWriteHalf) -> mpsc::UnboundedSender<Vec<u8>> {
    let (sender, mut reader) = 
        mpsc::unbounded_channel::<Vec<u8>>();

    tokio::spawn(async move {
        loop { if let Some(mut message) = reader.recv().await {
            if message == vec![0] {
                _ = writer.shutdown();
                reader.close();
                return;
            }
            _ = writer.write_all(&mut message).await;
        } }
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
        if let Err(err) = handle_packet(handler, &mut packet).await {
            eprintln!("runtime error handling packet: {:?}, client disconnected.", err);
            return;
        };
    }
}

async fn handle_packet(this: &mut ProtocolHandler, packet: &mut Packet) -> Result<(), RuntimeError> {
    let protocol = packet.read_u8()?;

    match this.status {
        States::HandShake => handle_handshake(this, packet, protocol),
        States::Status => status_response(protocol, packet, this).await,
        States::Login => login_response(protocol, packet, this).await,
        States::Configuration => configuration_response(protocol, packet, this).await,
        States::Play => play_responses(protocol, packet, this),
    }
}

fn handle_handshake( this: &mut ProtocolHandler, packet: &mut Packet, protocol: u8 ) -> Result<(), RuntimeError> {
    if !protocol == 0 { return Err(RuntimeError::IncorrectProtocol) };

    let protocol_version = packet.decode_varint()?;
    let used_ip = packet.decode_string()?;
    _ = packet.decode_ushort();

    let intent = packet.decode_varint()?;

    match intent {
        1 => this.status = States::Status,
        2 => this.status = States::Login,
        _ => return Err(RuntimeError::IncorrectIntent), // there exist intent 3 but it's ignored
    };

    Ok(())
}

