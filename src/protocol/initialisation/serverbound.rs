use std::{collections::HashMap, sync::{Arc, LazyLock}};
use crab_nbt::nbt;
use rand::random;
use crate::{
    protocol::{Player, RuntimeError, States, 
        datatypes::{Responses, Vector2, Vector3}, 
        initialisation::clientbound::{CLIENT_BOUND_PACKETS, default_registry_data}, 
        play::clientbound::CLIENT_BOUND_PACKETS as CLIENT_BOUND_PACKETS_PLAY, server::{server::{Data, SERVER}}
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
        let eid = SERVER.mutex.lock().get_push_eid();

        handler.player = Some(Player {
            eid: eid,
            username: username.clone(),
            uuid: uuid.clone(),
            keepalive_num: random(),
            hotbar: [0; 9],
            selected_slot: 0,
            position: Vector3 { x: 0.0, y: 128.0, z: 0.0 },
            rotation: Vector2 { x: 0.0, y: 0.0 }
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
        None });

    responses.insert( 0x03, |packet, handler| {
        let player = try_option_err!(handler.player.clone());
        println!("Player {} [{}] joined the game!", player.username, player.uuid);
        let mut players = Vec::new();
        let mut entity_packet = Vec::new();
        let lock = SERVER.mutex.lock();
        for player in lock.players.iter() {
            players.push((player.0.0.clone(), player.0.1.clone(), player.1.1));
            let player_info = match lock.positions.get(&player.1.1) {
                Some(t) => t,
                None => continue
            };
            entity_packet.extend(try_err!((CLIENT_BOUND_PACKETS_PLAY.summon_entity)(
                player.1.1, player.0.0.clone(), 149,
                player_info.0, player_info.1, 0,
                Vector3 { x: 0.0, y: 0.0, z: 0.0 }
            )));
        };
        // players.push();
        println!("{:?}", players);

        let response = [
            try_err!((CLIENT_BOUND_PACKETS_PLAY.login)(player.eid)),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.player_info_update)(
                [players, vec![(player.uuid.clone(), player.username.clone(), player.eid)]].concat())),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.game_event)(13, 0.0)),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.set_center_chunk)(0, 0)),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.keepalive)(player.keepalive_num)),
            entity_packet,
            {
                let mut output: Vec<u8> = Vec::new();
                for x in -5..5 {for y in -5..5 {
                    output.extend(try_err!((CLIENT_BOUND_PACKETS_PLAY.send_filled_chunk)(Vector2 { x, y })));
                }}
                output
            },
            // try_err!((CLIENT_BOUND_PACKETS_PLAY.chunk_batch_finish)(9)),
            try_err!((CLIENT_BOUND_PACKETS_PLAY.teleport_player)(1,
                player.position,
                Vector3 { x: 0.0, y: 1.0, z: 0.0 },
                player.rotation,
                None
            )),
        ].concat();
        _ = handler.writer.send(response);

        let message = format!("{} joined the game!", player.username);
        let message_bytes = try_err!((CLIENT_BOUND_PACKETS_PLAY.send_system_message)(nbt!("", {
            "text": message,
            "color": "yellow"
        }).write_unnamed().into(), false));

        let writer = handler.writer.clone();
        _ = SERVER.sender.send(Data::AddPlayer {
            player: (player.uuid, player.username),
            sender: writer.clone(),
            eid: player.eid,
            position: player.position,
            rotation: player.rotation,
        });
        _ = SERVER.sender.send(Data::Packet { data: message_bytes, filter: None });

        handler.status = States::Play;
        None
    });

    responses
}
