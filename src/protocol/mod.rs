mod initialisation;
mod play;
mod macros;
pub mod datatypes;
pub mod server;
pub mod cryptography;

use core::net::SocketAddr;
use std::{hash::Hash, panic::AssertUnwindSafe, sync::Arc};
use aes::cipher::{Array, consts::U1};
use cfb8::cipher::StreamCipher;
use futures::FutureExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, tcp::OwnedWriteHalf}, sync::{Mutex, mpsc}, time::Instant,
};

use crate::{
    datatypes::{Packet, decode_packet_length},
    protocol::{
        datatypes::{ Player, PlayerKey, ProtocolHandler, RuntimeError, States }, initialisation::serverbound::{configuration_response, login_response, status_response}, play::serverbound::play_responses, server::server::{Data, SERVER}
    },
};

pub fn protocol_handler_main(client: TcpStream, address: SocketAddr) {
    _ = tokio::spawn(async move {
        let (reader, writer) = client.into_split();
        let (writer, writer_upgrader) = setup_writer(writer).await;

        let mut this = ProtocolHandler {
            status: States::HandShake,
            reader, writer, writer_upgrader,
            protocol_version: 0,
            player: None,
            cipher: None,
        };

        let result = AssertUnwindSafe(
            protocol_handler(address, &mut this) // Catch user specific exceptions so server doesn't crash
        ).catch_unwind().await;

        match result {
            Ok(val) => (),
            Err(err) => {
                if let Some(error) = err.downcast_ref::<&str>()
                    { println!("got fatal error: {}", error) }
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

async fn setup_writer(mut writer: OwnedWriteHalf)
    -> (mpsc::UnboundedSender<Vec<u8>>,
        mpsc::Sender<cfb8::Encryptor<aes::Aes128>>) {
    let (sender, mut reader) = mpsc::unbounded_channel::<Vec<u8>>();
    let (cipher_sender, mut cipher_reader) = mpsc::channel::<cfb8::Encryptor<aes::Aes128>>(2);

    tokio::spawn(async move {
        let mut cipher: Option<Arc<Mutex<cfb8::Encryptor<aes::Aes128>>>> = None;
        loop { if let Some(mut message) = reader.recv().await {
            if cipher.is_none() && let Ok(encryptor) = cipher_reader.try_recv()
                { cipher = Some(Arc::new(Mutex::new(encryptor))) }

            if message == vec![0] {
                _ = writer.shutdown();
                reader.close();
                return;
            }

            if cipher.clone().is_none() { _ = writer.write_all(&mut message).await; continue; }

            let cipher_clone = cipher.clone().unwrap();
            let mut cipher = cipher_clone.lock().await;
            for chunk in message.chunks(2 << 10) {
                let mut out: Vec<u8> = vec![0u8; chunk.len()];
                cipher.encrypt_b2b(chunk, &mut out).unwrap();
                _ = writer.write_all(&out).await;
            }
        } }
    });

    (sender, cipher_sender)
}

async fn protocol_handler(address: SocketAddr, handler: &mut ProtocolHandler) {
    loop {
        let length = match decode_packet_length(&mut handler.reader, handler.cipher.clone()).await {
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

        if let Some(decryptor) = handler.cipher.clone() {
            for block in buffer.chunks_mut(1) { decryptor.lock().await .decrypt(block) }
        }

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
        States::Play => play_responses(protocol, packet, this).await,
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

