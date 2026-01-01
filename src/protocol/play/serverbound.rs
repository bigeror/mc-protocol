use crab_nbt::nbt;
use rand::random;
use std::time::Duration;
use std::{collections::HashMap, sync::LazyLock};

use tokio::time;

use crate::protocol::datatypes::Vector3;
use crate::protocol::play::clientbound::CLIENT_BOUND_PACKETS;
use crate::protocol::server::world::WORLD;
use crate::{
    protocol::{datatypes::{Responses, RuntimeError}}, try_err, try_option_err
};

pub static SERVERBOUND_PACKET_INSTANCE: LazyLock<Responses> = LazyLock::new(|| {
    let mut responses: Responses = HashMap::new();

    responses.insert( 0x1B, |packet, handler| {
            let arrived_keepalive = try_err!(packet.decode_long());
            let player = try_option_err!(handler.player.clone());
            if arrived_keepalive != player.keepalive_num { 
                return Some(RuntimeError::IncorrectKeepalive) 
            }

            let new_keepalive: i64 = random();
            try_option_err!(handler.player.as_mut()).keepalive_num = new_keepalive;
            let writer = handler.writer.clone();

            _ = tokio::spawn(async move {
                time::sleep(Duration::from_secs(5)).await;
                let response = try_err!((CLIENT_BOUND_PACKETS.keepalive)(new_keepalive));
                _ = writer.send(response);
                None
            });

            None
        }
    );

    responses.insert( 0x08, |packet, handler| {
            let message = try_err!(packet.decode_string());
            let username: &str = &try_option_err!(handler.player.clone()).username;

            println!("{} -> {}", username, &message);

            let response = try_err!((CLIENT_BOUND_PACKETS.send_system_message)(nbt!("", {
                "text": "",
                "extra": [
                    {"text": username, "italic": true},
                    {"text": ": ", "color": "gray"},
                    {"text": message}
                ],
            }).write_unnamed().into(), false));

            _ = handler.writer.send(response);
            None
        }
    );

    responses.insert( 0x28, |packet, handler| {
            let status = try_err!(packet.decode_varint());
            let position = try_err!(packet.decode_position());
            let face = try_err!(packet.read_u8());
            let sequence = try_err!(packet.decode_varint());

            let mut world = WORLD.lock().unwrap();
            world.replace_block(position, 0);

            let response = [
                try_err!((CLIENT_BOUND_PACKETS.block_update)(0, position)),
                try_err!((CLIENT_BOUND_PACKETS.aknowledge_block_change)(sequence)),
            ].concat();
            _ = handler.writer.send(response);
            None
        }
    );

    responses.insert( 0x3F, |packet, handler| {
            let hand = try_err!(packet.decode_varint());
            let location = try_err!(packet.decode_position());
            let face = try_err!(packet.decode_varint());
            let cursor = Vector3 {
                x: try_err!(packet.decode_float()),
                y: try_err!(packet.decode_float()),
                z: try_err!(packet.decode_float()),
            };
            let inside_block = try_err!(packet.read_u8());
            let world_border_hit = try_err!(packet.read_u8());
            let sequence = try_err!(packet.decode_varint());

            let pos_offset = [
                Vector3 {x:0, y:-1, z:0},
                Vector3 {x:0, y:1, z:0},
                Vector3 {x:0, y:0, z:-1},
                Vector3 {x:0, y:0, z:1},
                Vector3 {x:-1, y:0, z:0},
                Vector3 {x:1, y:0, z:0},
            ][face as usize];
            let actual_pos = Vector3::add(location, pos_offset);

            let mut world = WORLD.lock().unwrap();
            world.replace_block(actual_pos, 1);

            let response = [
                try_err!((CLIENT_BOUND_PACKETS.block_update)(1, actual_pos)),
                try_err!((CLIENT_BOUND_PACKETS.aknowledge_block_change)(sequence)),
            ].concat();

            _ = handler.writer.send(response);
            None
        }
    );

    responses
});
