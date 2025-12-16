use crab_nbt::nbt;
use rand::random;
use std::time::Duration;
use std::{collections::HashMap, sync::LazyLock};

use tokio::time;

use crate::datatypes::Double;
#[allow(unused_imports)]
use crate::datatypes::{Float, Long, Position, StringBuffer, VarInt};
use crate::protocol::datatypes::{Vector2, Vector3};
use crate::protocol::play::clientbound::CLIENT_BOUND_PACKETS;
use crate::protocol::server::server::SERVER;
use crate::{
    protocol::{datatypes::{Responses, RuntimeError}}, try_err, try_option_err
};

pub static SERVERBOUND_PACKET_INSTANCE: LazyLock<Responses> = LazyLock::new(|| {
    let mut responses: Responses = HashMap::new();

    responses.insert(
        0x1B,
        |packet, handler| {
            let arrived_keepalive = try_err!(Long(packet).decode(1)).value;
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

    responses.insert( 0x08,
    |packet, handler| {
        let message = try_err!(StringBuffer(packet).decode(1)).value;
        let username = try_option_err!(handler.player.clone()).username.to_string();

        println!("{} -> {}", username, message);

        let response = try_err!((CLIENT_BOUND_PACKETS.send_system_message)(nbt!("", {
            "text": username, "extra": [
            {"text": " | ", "color": "dark_gray"},
            {"text": message},
        ]}).write_unnamed().into(), false));

        SERVER.lock().unwrap().send_to_players(response, None);
        None
    });

    responses.insert(0x1D, |packet, handler| {
        let x = try_err!(Double(packet).decode(1));
        let y = try_err!(Double(packet).decode(x.offset));
        let z = try_err!(Double(packet).decode(y.offset));

        let response = try_err!((CLIENT_BOUND_PACKETS.teleport_player)(0, Vector3 {x:0.0, y:128.0, z:0.0}, Vector3 {x:0.0, y:0.0, z:0.0}, Vector2 {x:0.0, y:0.0}, None));
        if x.value.abs() > 0.2 || (y.value - 128.0).abs() > 0.2 || z.value.abs() > 0.2 {_ = handler.writer.send(response)};

        None
    });

    responses.insert(0x1E, |packet, handler| {
        let player = try_option_err!(handler.player.as_mut());

        let x = try_err!(Double(packet).decode(1));
        let y = try_err!(Double(packet).decode(x.offset));
        let z = try_err!(Double(packet).decode(y.offset));
        let yaw = try_err!(Float(packet).decode(z.offset));
        let pitch = try_err!(Float(packet).decode(yaw.offset));
        let response = try_err!((CLIENT_BOUND_PACKETS.teleport_player)(0, Vector3 {x:0.0, y:128.0, z:0.0}, Vector3 {x:0.0, y:0.0, z:0.0}, Vector2 {x:0.0, y:0.0}, None));
        if x.value.abs() > 0.2 || (y.value - 128.0).abs() > 0.2 || z.value.abs() > 0.2 || yaw.value != 0.0 || pitch.value != 0.0 {_ = handler.writer.send(response)};

        player.rotation.x += yaw.value * 0.1;
        player.rotation.y += pitch.value * 0.1;
        let game = try_option_err!(handler.game.as_mut()).clone();
        let player_copy = player.clone();
        tokio::spawn(async move {game.lock().await.player = player_copy});
        None
    });

    responses.insert(0x1F, |packet, handler| {
        let player = try_option_err!(handler.player.as_mut());

        let yaw = try_err!(Float(packet).decode(1));
        let pitch = try_err!(Float(packet).decode(yaw.offset));
        let response = try_err!((CLIENT_BOUND_PACKETS.teleport_player)(0, Vector3 {x:0.0, y:128.0, z:0.0}, Vector3 {x:0.0, y:0.0, z:0.0}, Vector2 {x:0.0, y:0.0}, None));
        if yaw.value != 0.0 || pitch.value != 0.0 {_ = handler.writer.send(response)};

        player.rotation.x += yaw.value * 0.1;
        player.rotation.y += pitch.value * 0.1;
        let game = try_option_err!(handler.game.as_mut()).clone();
        let player_copy = player.clone();
        tokio::spawn(async move {game.lock().await.player = player_copy});
        None
    });

    responses
});
