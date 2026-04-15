use std::sync::Arc;
use aes::cipher::KeyIvInit;
use cfb8::{Decryptor, Encryptor};
use crab_nbt::nbt;
use rand::{Rng, random};
use rsa::Pkcs1v15Encrypt;
use tokio::sync::Mutex;
use crate::{datatypes::Packet, protocol::{Player, States, cryptography::ENCRYPT_KEY_PAIR, datatypes::{Display, PlayerKey, ProtocolHandler,  RuntimeError, Vector2, Vector3}, initialisation::clientbound::{CLIENT_BOUND_PACKETS, default_registry_data}, play::clientbound::CLIENT_BOUND_PACKETS as CLIENT_BOUND_PACKETS_PLAY, server::{server::{Data, SERVER}, world::WORLD}}};

pub async fn status_response(protocol: u8, packet: &mut Packet, handler: &mut ProtocolHandler)
    -> Result<(), RuntimeError> {
    match protocol {
    0x00 => {
        let response = (CLIENT_BOUND_PACKETS.status.status_response)()?;
        _ = handler.writer.send(response);
        Ok(())
    },

    0x01 => {
        let response = (CLIENT_BOUND_PACKETS.status.ping_response)(packet.decode_long()?)?;
        _ = handler.writer.send(response);
        Ok(())
    },
    _ => Err(RuntimeError::IncorrectProtocol)
    }
}

pub async fn login_response(protocol: u8, packet: &mut Packet, handler: &mut ProtocolHandler)
    -> Result<(), RuntimeError> {
    match protocol {
    0x00 => {
        let username: Arc<str> = packet.decode_string()?.into();
        let uuid = packet.decode_uuid()?;
        let eid = SERVER.mutex.lock().await.get_push_eid();

        let mut verify_token = [0u8; 64];
        rand::rng().fill(&mut verify_token);

        handler.player = Some(Player {
            eid: eid,
            username: username.clone(),
            uuid: uuid,
            keepalive_num: random(),
            hotbar: [0; 9],
            selected_slot: 0,
            position: Vector3 { x: 0.0, y: 128.0, z: 0.0 },
            rotation: Vector2 { x: 0.0, y: 0.0 },
            verify_token: verify_token,
        });

        let response = (CLIENT_BOUND_PACKETS.connect.encryption_request)(
            ENCRYPT_KEY_PAIR.public_key_der.clone(),
            verify_token.to_vec(), false
        )?;

        _ = handler.writer.send(response);
        Ok(())
    },

    0x01 => { // encryption response (auth + enable encrypt)
        let shared_secret_length = packet.decode_varint()? as usize;
        let shared_secret_encrypted = packet.read_buf(shared_secret_length)?;
        let shared_secret = ENCRYPT_KEY_PAIR.private_key
            .decrypt(Pkcs1v15Encrypt, &shared_secret_encrypted)
            .map_err(|_| RuntimeError::IncorrectEncryptionResponse)?;
        if shared_secret.len() != 16 { return Err(RuntimeError::IncorrectEncryptionResponse) }

        let verify_token_encrypted_length = packet.decode_varint()? as usize;
        let verify_token_encrypted = packet.read_buf(verify_token_encrypted_length)?;
        let verify_token = ENCRYPT_KEY_PAIR.private_key
            .decrypt(Pkcs1v15Encrypt, &verify_token_encrypted)?;

        if verify_token != handler.player.clone().ok_or(RuntimeError::UnexpectedNone)?
            .verify_token { return Err(RuntimeError::IncorrectEncryptionResponse) }

        // let hash: [u8; 20] = sha1::Sha1::new()
        //     .chain_update(shared_secret.clone())
        //     .chain_update(&ENCRYPT_KEY_PAIR.public_key_der)
        //     .finalize().into();
        // let hex = num_bigint::BigInt::from_signed_bytes_be(&hash).to_str_radix(16);

        // upgrade the sender
        _ = handler.writer_upgrader.send(Encryptor::<aes::Aes128>::new_from_slices(&shared_secret, &shared_secret)
            .map_err(|_| RuntimeError::IncorrectEncryptionResponse)?).await;

        // save the decryptor to use afterwards
        handler.cipher = Some(Arc::new(Mutex::new(Decryptor::<aes::Aes128>::new_from_slices(&shared_secret, &shared_secret)
                .map_err(|_| RuntimeError::IncorrectEncryptionResponse)?)));

        let player = handler.player.clone().ok_or(RuntimeError::UnexpectedNone)?;
        let response = (CLIENT_BOUND_PACKETS.connect.login_success)(player.username, player.uuid)?;
        _ = handler.writer.send(response);
        Ok(())
    },

    0x03 => {
        handler.status = States::Configuration;
        Ok(())
    },
    _ => Err(RuntimeError::IncorrectProtocol)
    }
}

pub async fn configuration_response(protocol: u8, packet: &mut Packet, handler: &mut ProtocolHandler)
    -> Result<(), RuntimeError> {
    match protocol {
    0x00 => {
        let response = [
            (CLIENT_BOUND_PACKETS.connect.plugin_message)()?,
            (CLIENT_BOUND_PACKETS.connect.send_datapacks)()?,
        ].concat();
        _ = handler.writer.send(response);
        Ok(())
    },

    0x07 => {
        let response = [
            default_registry_data()?,
            (CLIENT_BOUND_PACKETS.connect.configuration_finish)()?,
        ].concat();
        _ = handler.writer.send(response);
        Ok(())
    },

    0x03 => {
        let player = handler.player.clone().ok_or(RuntimeError::UnexpectedNone)?;
        let mut players = Vec::new();
        let mut entity_packet = Vec::new();
        let lock = SERVER.mutex.lock().await;

        for player in lock.players.iter() {
            players.push(player.0.clone());

            entity_packet.extend((CLIENT_BOUND_PACKETS_PLAY.summon_entity)(
                player.1.eid, player.0.uuid.clone(), 149,
                player.1.position, player.1.rotation, 0,
                Vector3 { x: 0.0, y: 0.0, z: 0.0 }
            )?);
        };
        let player_name: &str = &player.username.clone();
        let player_uuid: &str = &player.uuid.display();

        let response = [
            (CLIENT_BOUND_PACKETS_PLAY.login)(player.eid)?,
            (CLIENT_BOUND_PACKETS_PLAY.player_info_update)(
                [players, vec![PlayerKey::from(&player)]].concat())?,
            (CLIENT_BOUND_PACKETS_PLAY.game_event)(13, 0.0)?,
            (CLIENT_BOUND_PACKETS_PLAY.set_center_chunk)(0, 0)?,
            (CLIENT_BOUND_PACKETS_PLAY.keepalive)(player.keepalive_num)?,
            entity_packet,
            {
                let mut output: Vec<u8> = Vec::new();
                for x in -5..5 {for y in -5..5 {
                    let mut world = WORLD.lock().await;
                    output.extend((CLIENT_BOUND_PACKETS_PLAY.send_filled_chunk)(Vector2 { x, y }, &mut world)?);
                }}
                output
            },
            // (CLIENT_BOUND_PACKETS_PLAY.chunk_batch_finish)(9)),
            (CLIENT_BOUND_PACKETS_PLAY.teleport_player)(1,
                player.position,
                Vector3 { x: 0.0, y: 1.0, z: 0.0 },
                player.rotation,
                None
            )?,
            (CLIENT_BOUND_PACKETS_PLAY.send_system_message)(nbt!("", {
                "text": "",
                "extra": [
                    {"text": "[", "color": "gray"},
                    {"text": "+", "color": "green"},
                    {"text": "] ", "color": "gray"},
                    {"text": player_name, "hover_event": {"action": "show_text", "value": player_uuid}}
                ]
            }).write_unnamed().to_vec(), false)?,
        ].concat();
        _ = handler.writer.send(response);

        let writer = handler.writer.clone();
        _ = SERVER.sender.send(Data::AddPlayer {
            player: PlayerKey::from(&player),
            sender: writer.clone(),
            eid: player.eid,
            position: player.position,
            rotation: player.rotation,
        });

        handler.status = States::Play;
        Ok(())
    },
    _ => Ok(())
    }
}
