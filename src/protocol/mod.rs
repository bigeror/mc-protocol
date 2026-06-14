mod initialisation;
mod play;
mod macros;
pub mod datatypes;
pub mod server;
pub mod cryptography;

use core::net::SocketAddr;
use std::{collections::VecDeque, mem, panic::AssertUnwindSafe, sync::Arc};
use aes::Aes128;
use cfb8::Encryptor;
use futures::FutureExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, tcp::OwnedWriteHalf}, sync::{Mutex, mpsc},
};

use crate::{
    datatypes::{Packet, decode_packet_length},
    protocol::{
        datatypes::{ Player, PlayerKey, ProtocolHandler, RuntimeError, SendPacket, States }, initialisation::serverbound::{configuration_response, login_response, status_response}, play::serverbound::play_responses, server::server::{Data, SERVER}
    },
};

pub fn protocol_handler_main(client: TcpStream, address: SocketAddr) {
    _ = tokio::spawn(async move {
        let (reader, writer) = client.into_split();
        let writer = setup_writer(writer).await;

        let mut this = ProtocolHandler {
            status: States::HandShake,
            reader, writer,
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
        _ = this.writer.send(SendPacket::TerminateSender);
        return;
    })
}

async fn setup_writer(mut writer: OwnedWriteHalf)
    -> mpsc::UnboundedSender<SendPacket> {
    let (sender, mut reader) = mpsc::unbounded_channel::<SendPacket>();
    let cipher: Arc<Mutex<Option<cfb8::Encryptor<aes::Aes128>>>> = Arc::new(Mutex::new(None));
    let cipher_clone = cipher.clone();
    let message_stack: Arc<Mutex<VecDeque<Arc<Vec<u8>>>>> = Arc::new(Mutex::new(VecDeque::new()));
    let message_stack_clone = message_stack.clone();
    let low_priority_message_stack: Arc<Mutex<VecDeque<Arc<Vec<u8>>>>> = Arc::new(Mutex::new(VecDeque::new()));
    let low_priority_message_stack_clone = low_priority_message_stack.clone();
    // true -> notification arrived, false -> process termination signal
    let (send_packet_notifications, mut receive_packet_notifications) = mpsc::unbounded_channel::<bool>();

    // sender
    tokio::spawn(async move {
        loop { if let Some(notification) = receive_packet_notifications.recv().await {
            // handle kill process
            if !notification {
                _ = writer.shutdown();
                receive_packet_notifications.close();
                return;
            }

            let mut message_stack = message_stack_clone.lock().await;
            let mut lp_stack = low_priority_message_stack_clone.lock().await;
            let mut message = &message_stack.pop_front()
                .unwrap_or_else(move || lp_stack.pop_front().expect("invalid state in notifications system"));

            match cipher_clone.lock().await.as_mut() as Option<&mut Encryptor<Aes128>> {
                None => {
                    _ = writer.write_all(&mut message).await;
                    continue;
                },
                Some(cipher) => {
                    for chunk in message.chunks(2 << 5) {
                        let mut out: Vec<u8> = vec![0u8; chunk.len()];
                        cipher.encrypt_b2b(chunk, &mut out).unwrap();
                        _ = writer.write_all(&out).await;
                    }
                }
            }
        } }
    });

    // receiver
    tokio::spawn(async move {
        loop { if let Some(packet) = reader.recv().await {
            match packet {
                SendPacket::SendPacket(message) => {
                    message_stack.lock().await.push_back(Arc::new(message.clone()));
                    _ = send_packet_notifications.send(true);
                },
                SendPacket::UpgradeSender(encryptor) => {
                    _ = mem::replace(
                        &mut cipher.lock().await as &mut Option<cfb8::Encryptor<aes::Aes128>>,
                        Some(encryptor)
                    );
                },
                SendPacket::TerminateSender => {
                    _ = send_packet_notifications.send(false);
                    _ = reader.close();
                    return;
                },
                SendPacket::LowPriority(message) => {
                    low_priority_message_stack.lock().await.push_back(Arc::new(message));
                    _ = send_packet_notifications.send(true);
                },
            }
        } }
    });

    sender
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

