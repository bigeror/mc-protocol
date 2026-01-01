use std::{collections::HashMap, sync::{Arc, LazyLock}};
use rand::random;

use crate::{
    protocol::{Player, RuntimeError, States, 
        datatypes::{Responses, Vector2, Vector3}, 
        initialisation::clientbound::{CLIENT_BOUND_PACKETS, default_registry_data}, 
        play::clientbound::CLIENT_BOUND_PACKETS as CLIENT_BOUND_PACKETS_PLAY
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

pub static SERVER_BOUND_PACKETS_INSTANCE: LazyLock<ServerBoundPackets> = LazyLock::new(ServerBoundPackets::new);

fn status_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert( 0x00, |packet, handler| {
        let response = try_err!((CLIENT_BOUND_PACKETS.status.status_response)());
        _ = handler.writer.send(response);
        None
    });

    responses.insert( 0x01, |packet, handler| {
        let response = try_err!((CLIENT_BOUND_PACKETS.status.ping_response)(try_err!(packet.decode_long())));
        _ = handler.writer.send(response);
        None
    });

    responses
}

fn login_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert( 0x00, |packet, handler| {
        let username: Arc<str> = try_err!(packet.decode_string()).into();
        let uuid: Arc<str> = try_err!(packet.decode_uuid()).into();

        handler.player = Some(Player {
            username: username.clone(),
            uuid: uuid.clone(),
            keepalive_num: random(),
        });

        let response = try_err!((CLIENT_BOUND_PACKETS.connect.login_success)(username, uuid));
        _ = handler.writer.send(response);
        None
    });

    responses.insert( 0x03, |packet, handler| {
        handler.status = States::Configuration;
        None
    });

    responses
}

fn configuration_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert( 0x00, |packet, handler| {
        let response = [
            try_err!((CLIENT_BOUND_PACKETS.connect.plugin_message)()),
            try_err!((CLIENT_BOUND_PACKETS.connect.send_datapacks)()),
        ].concat();
        _ = handler.writer.send(response);
        None
    });

    responses.insert( 0x07, |packet, handler| {
        let response = [
            try_err!(default_registry_data()),
            try_err!((CLIENT_BOUND_PACKETS.connect.configuration_finish)()),
        ].concat();
        _ = handler.writer.send(response);
        None
    });

    responses.insert( 0x03, |packet, handler| {
        let player = try_option_err!(handler.player.clone());
        println!("Player {} [{}] joined the game!", player.username, player.uuid);

        let response = [
            try_err!((CLIENT_BOUND_PACKETS_PLAY.login)()),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.player_info_update)(player.uuid, player.username)),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.game_event)(13, 0.0)),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.teleport_player)(1,
                Vector3 { x: 0.0, y: 128.0, z: 0.0 },
                Vector3 { x: 0.0, y: 1.0, z: 0.0 },
                Vector2 { x: 0.0, y: 0.0 },
                None
            )),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.set_center_chunk)(0, 0)),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.keepalive)(player.keepalive_num)),
            {
                let mut output: Vec<u8> = Vec::new();
                for x in -5..5 {for y in -5..5 {
                    output.extend(try_err!((CLIENT_BOUND_PACKETS_PLAY.send_filled_chunk)(Vector2 { x, y })));
                }}
                output
            },
            try_err!((CLIENT_BOUND_PACKETS_PLAY.chunk_batch_finish)(9)),
        ].concat();
        _ = handler.writer.send(response);

        handler.status = States::Play;
        None
    });

    responses
}
