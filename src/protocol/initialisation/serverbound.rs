use std::{collections::HashMap, sync::{Arc, LazyLock}};
use crab_nbt::nbt;
use rand::random;
use crate::protocol::{Player, States, 
        datatypes::{Display, PlayerKey, Responses, RuntimeError, Vector2, Vector3}, 
        initialisation::clientbound::{CLIENT_BOUND_PACKETS, default_registry_data}, 
        play::clientbound::CLIENT_BOUND_PACKETS as CLIENT_BOUND_PACKETS_PLAY, server::server::{Data, SERVER}
    };

pub struct ServerBoundPackets {
    pub status: Responses,
    pub login: Responses,
    pub configuration: Responses,
    pub play: Responses,
}

impl ServerBoundPackets {
    fn new() -> Self {
        ServerBoundPackets {
            status: status_responses(),
            login: login_responses(),
            configuration: configuration_responses(),
            play: HashMap::new(),
        }
    }
}

pub static SERVER_BOUND_PACKETS_INSTANCE: LazyLock<ServerBoundPackets> = LazyLock::new(ServerBoundPackets::new);

fn status_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert( 0x00, |packet, handler| {
        let response = (CLIENT_BOUND_PACKETS.status.status_response)()?;
        _ = handler.writer.send(response);
        Ok(())
    });

    responses.insert( 0x01, |packet, handler| {
        let response = (CLIENT_BOUND_PACKETS.status.ping_response)(packet.decode_long()?)?;
        _ = handler.writer.send(response);
        Ok(())
    });

    responses
}

fn login_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert( 0x00, |packet, handler| {
        let username: Arc<str> = packet.decode_string()?.into();
        let uuid = packet.decode_uuid()?;
        let eid = SERVER.mutex.lock().get_push_eid();

        handler.player = Some(Player {
            eid: eid,
            username: username.clone(),
            uuid: uuid,
            keepalive_num: random(),
            hotbar: [0; 9],
            selected_slot: 0,
            position: Vector3 { x: 0.0, y: 128.0, z: 0.0 },
            rotation: Vector2 { x: 0.0, y: 0.0 }
        });

        let response = (CLIENT_BOUND_PACKETS.connect.login_success)(username, uuid)?;
        _ = handler.writer.send(response);
        Ok(())
    });

    responses.insert( 0x03, |packet, handler| {
        handler.status = States::Configuration;
        Ok(())
    });

    responses
}

fn configuration_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert( 0x00, |packet, handler| {
        let response = [
            (CLIENT_BOUND_PACKETS.connect.plugin_message)()?,
            (CLIENT_BOUND_PACKETS.connect.send_datapacks)()?,
        ].concat();
        _ = handler.writer.send(response);
        Ok(())
    });

    responses.insert( 0x07, |packet, handler| {
        let response = [
            default_registry_data()?,
            (CLIENT_BOUND_PACKETS.connect.configuration_finish)()?,
        ].concat();
        _ = handler.writer.send(response);
        Ok(())
    });

    responses.insert( 0x03, |packet, handler| {
        let player = handler.player.clone().ok_or(RuntimeError::UnexpectedNone)?;
        let mut players = Vec::new();
        let mut entity_packet = Vec::new();
        let lock = SERVER.mutex.lock();

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
                    output.extend((CLIENT_BOUND_PACKETS_PLAY.send_filled_chunk)(Vector2 { x, y })?);
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
    });

    responses
}
