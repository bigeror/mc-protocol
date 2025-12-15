use std::{collections::HashMap, sync::{Arc, LazyLock}};
use rand::random;
use crab_nbt::nbt;

use crate::{
    datatypes::{StringBuffer, UUID}, protocol::{Player, RuntimeError, States, 
        datatypes::{Responses, Vector2, Vector3}, 
        initialisation::clientbound::{CLIENT_BOUND_PACKETS, default_registry_data}, 
        play::clientbound::CLIENT_BOUND_PACKETS as CLIENT_BOUND_PACKETS_PLAY, server::server::SERVER
    }, try_err, try_option_err
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

pub static SERVER_BOUND_PACKETS_INSTANCE: LazyLock<ServerBoundPackets> =
    LazyLock::new(ServerBoundPackets::new);

fn status_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert(
        0x00,
        |packet, handler| {
            let response = try_err!((CLIENT_BOUND_PACKETS.status.status_response)());
            _ = handler.writer.send(response);
            None
        },
    );

    responses.insert(
        0x01,
        |packet, handler| {
            let response = try_err!((CLIENT_BOUND_PACKETS.status.ping_response)(packet));
            _ = handler.writer.send(response);
            None
        },
    );

    responses
}

fn login_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert(
        0x00,
        |packet, handler| {
            let username_raw = try_err!(StringBuffer(packet).decode(1));
            let uuid_raw = try_err!(UUID(packet).decode(username_raw.offset));

            let username: Arc<str> = username_raw.value.into();
            let uuid: Arc<str> = uuid_raw.value.into();

            handler.player = Some(Player {
                username: username.clone(),
                uuid: uuid.clone(),
                keepalive_num: random(),
            });

            let response = try_err!((CLIENT_BOUND_PACKETS.connect.login_success)(username, uuid));
            _ = handler.writer.send(response);
            None
        },
    );

    responses.insert(
        0x03,
        |packet, handler| {
            handler.status = States::Configuration;
            None
        }
    );

    responses
}

fn configuration_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert(
        0x00,
        |packet, handler| {
            let response = [
                try_err!((CLIENT_BOUND_PACKETS.connect.plugin_message)()),
                try_err!((CLIENT_BOUND_PACKETS.connect.send_datapacks)()),
            ].concat();
            _ = handler.writer.send(response);
            None
        }
    );

    responses.insert(
        0x07, 
        |packet, handler| {
            let response = [
                try_err!(default_registry_data()),
                try_err!((CLIENT_BOUND_PACKETS.connect.configuration_finish)()),
            ].concat();
            _ = handler.writer.send(response);
            None
        }
    );

    responses.insert(
        0x03, 
        |packet, handler| {
            let player = try_option_err!(handler.player.clone());
            println!("Player {}[{}] joined the game!", player.username, player.uuid);

            let response = [
                try_err!((CLIENT_BOUND_PACKETS_PLAY.login)()),
                try_err!((CLIENT_BOUND_PACKETS_PLAY.player_info_update)(player.clone().uuid, player.clone().username)),
                try_err!((CLIENT_BOUND_PACKETS_PLAY.game_event)(13, 0.0)),
                try_err!((CLIENT_BOUND_PACKETS_PLAY.teleport_player)(1,
                    Vector3 { x: 0.0, y: 128.0, z: 0.0 },
                    Vector3 { x: 0.0, y: 1.0, z: 0.0 },
                    Vector2 { x: 0.0, y: 0.0 },
                    None
                )),
                try_err!((CLIENT_BOUND_PACKETS_PLAY.set_center_chunk)(0, 0)),
                // try_err!((CLIENT_BOUND_PACKETS_PLAY.chunk_batch_start)()),
                try_err!((CLIENT_BOUND_PACKETS_PLAY.keepalive)(player.keepalive_num))
            ].concat();
            _ = handler.writer.send(response);
            for x in -3..3 {for y in -3..3 {
                _ = handler.writer.send(try_err!((CLIENT_BOUND_PACKETS_PLAY.send_filled_chunk)(Vector2 { x, y })));
            }}
            _ = handler.writer.send(try_err!((CLIENT_BOUND_PACKETS_PLAY.chunk_batch_finish)(9)));

            let mut server = SERVER.lock().unwrap();
            server.players.insert((player.clone().uuid, player.clone().username), handler.writer.clone());

            let join_message = format!("{} joined the game!", player.username);
            server.send_to_players(try_err!((CLIENT_BOUND_PACKETS_PLAY.send_system_message)(nbt!("", {
                "text": join_message,
                "color": "yellow",
            }).write_unnamed().to_vec(), false)), None);

            handler.status = States::Play;
            None
        }
    );

    responses
}
