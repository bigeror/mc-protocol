use crab_nbt::nbt;
use rand::random;
use std::time::Duration;
use std::{collections::HashMap, sync::LazyLock};

use tokio::time;

use crate::protocol::datatypes::{Display, PlayerKey, Vector2, Vector3};
use crate::protocol::play::clientbound::CLIENT_BOUND_PACKETS;
use crate::protocol::play::place_block::get_block_id;
use crate::protocol::server::mapping::MAP;
use crate::protocol::server::server::{Data, SERVER};
use crate::protocol::server::world::WORLD;
use crate::{
    protocol::{datatypes::{Responses, RuntimeError}},
};

pub static SERVERBOUND_PACKET_INSTANCE: LazyLock<Responses> = LazyLock::new(|| {
    let mut responses: Responses = HashMap::new();

    responses.insert( 0x1B, |packet, handler| {
        let arrived_keepalive = packet.decode_long()?;
        let player = handler.player.clone().ok_or(RuntimeError::UnexpectedNone)?;
        if arrived_keepalive != player.keepalive_num { 
            return Err(RuntimeError::IncorrectKeepalive) 
        }

        let new_keepalive: i64 = random();
        handler.player.as_mut().ok_or(RuntimeError::UnexpectedNone)?.keepalive_num = new_keepalive;
        let writer = handler.writer.clone();

        _ = tokio::spawn(async move {
            time::sleep(Duration::from_secs(5)).await;
            let response = match (CLIENT_BOUND_PACKETS.keepalive)(new_keepalive) {
                Ok(val) => val,
                Err(e) => return
            };
            _ = writer.send(response);
        });

        Ok(())
    });

    responses.insert( 0x08, |packet, handler| {
        let message = packet.decode_string()?;
        let player = handler.player.clone().ok_or(RuntimeError::UnexpectedNone)?;
        let username: &str = &player.username;
        let uuid: &str = &player.uuid.display();

        println!("{} -> {}", username, &message);

        let response = (CLIENT_BOUND_PACKETS.send_system_message)(nbt!("", {
            "text": "",
            "extra": [
                {"text": username, "hover_event": {"action": "show_text", "value": uuid}},
                {"text": " > ", "color": "gray"},
                {"text": message}
            ],
        }).write_unnamed().into(), false)?;

        _ = SERVER.sender.send(Data::Packet { data: response, filter: None });
        Ok(())
    });

    responses.insert( 0x28, |packet, handler| {
        let status = packet.decode_varint()?;
        let position = packet.decode_position()?;
        let face = packet.read_u8()?;
        let sequence = packet.decode_varint()?;

        let mut world = WORLD.lock();
        world.replace_block(position, 0);
        drop(world);

        let player = handler.player.as_ref().ok_or(RuntimeError::UnexpectedNone)?;

        _ = handler.writer.send([
            (CLIENT_BOUND_PACKETS.block_update)(0, position)?,
            (CLIENT_BOUND_PACKETS.aknowledge_block_change)(sequence)?,
        ].concat());

        let response = [
            (CLIENT_BOUND_PACKETS.block_update)(0, position)?,
            (CLIENT_BOUND_PACKETS.entity_animation)(player.eid, 0)?,
        ].concat();

        _ = SERVER.sender.send(Data::Packet { data: response, filter: Some(PlayerKey::from(player)) });

        Ok(())
    });

    responses.insert( 0x3F, |packet, handler| {
        let hand = packet.decode_varint()?;
        let location = packet.decode_position()?;
        let face = packet.decode_varint()?;
        if face < 0 || face >= 6 {return Err(RuntimeError::IncorrectValue)}
        let cursor = Vector3 {
            x: packet.decode_float()?,
            y: packet.decode_float()?,
            z: packet.decode_float()?,
        };
        let inside_block = packet.read_u8()?;
        let world_border_hit = packet.read_u8()?;
        let sequence = packet.decode_varint()?;

        let face = [
            Vector3 {x:0, y:-1, z:0},
            Vector3 {x:0, y:1, z:0},
            Vector3 {x:0, y:0, z:-1},
            Vector3 {x:0, y:0, z:1},
            Vector3 {x:-1, y:0, z:0},
            Vector3 {x:1, y:0, z:0},
        ][face as usize];

        let player = handler.player.clone().ok_or(RuntimeError::UnexpectedNone)?;
        let item = player.hotbar[player.selected_slot as usize];
        let block = match MAP.item_to_block.get(&item) {
            Some(block) => block,
            None => return Ok(()), // if no block is selected we can't place any block.
        };

        let (block_id, actual_pos) = get_block_id(location, cursor, face, block);
        let block_change = (CLIENT_BOUND_PACKETS.block_update)(block_id, actual_pos)?;

        let mut world = WORLD.lock();
        world.replace_block(actual_pos, block_id);
        drop(world);

        _ = handler.writer.send([
            block_change.clone(),
            (CLIENT_BOUND_PACKETS.aknowledge_block_change)(sequence)?,
        ].concat());

        _ = SERVER.sender.send(Data::Packet { data: [
            block_change,
            (CLIENT_BOUND_PACKETS.entity_animation)(player.eid, 0)?,
        ].concat(), filter: Some(PlayerKey::from(&player)) });
        Ok(())
    });

    responses.insert(0x37, |packet, handler| {
        let slot = packet.decode_short()? - 36;
        if slot < 0 || slot > 8 {return Ok(())} // listen only for hotbar changes
        let player = handler.player.as_mut().ok_or(RuntimeError::UnexpectedNone)?;

        let count = packet.decode_varint()?;
        if count == 0 {
            player.hotbar[slot as usize] = 0;
            if player.selected_slot == slot {_ = SERVER.sender.send(Data::Packet{
                data: (CLIENT_BOUND_PACKETS.set_equipment)(player.eid, vec![(0, count, 0)])?,
                filter: Some(PlayerKey::from(&player))
            })}
            return Ok(());
        }

        let id = packet.decode_varint()?;
        player.hotbar[slot as usize] = id;

        if player.selected_slot == slot {_ = SERVER.sender.send(Data::Packet{
            data: (CLIENT_BOUND_PACKETS.set_equipment)(player.eid, vec![(0, count, id)])?,
            filter: Some(PlayerKey::from(&player))
        })}

        Ok(())
    });

    responses.insert(0x34, |packet, handler| {
        let slot = packet.decode_short()?;
        if slot < 0 || slot > 8 { return Err(RuntimeError::IncorrectValue); }
        let player = handler.player.as_mut().ok_or(RuntimeError::UnexpectedNone)?;
        player.selected_slot = slot;

        let item = player.hotbar[slot as usize];
        _ = SERVER.sender.send(Data::Packet {
            data: (CLIENT_BOUND_PACKETS.set_equipment)(player.eid, vec![(0, if item != 0 {1} else {0}, item)])?,
            filter: Some(PlayerKey::from(&player))
        });
        Ok(())
    });

    responses.insert(0x1D, |packet, handler| {
        let position = Vector3 {
            x: packet.decode_double()?,
            y: packet.decode_double()?,
            z: packet.decode_double()?,
        };

        let flags = packet.read_u8()?;
        let player = handler.player.as_mut().ok_or(RuntimeError::UnexpectedNone)?;
        player.position = position;

        _ = SERVER.sender.send(Data::UpdatePosition {
            player_key: PlayerKey::from(&player),
            position, rotation: player.rotation,
            on_ground: (flags & 0x01) != 0
        });
        Ok(())
    });

    responses.insert(0x1E, |packet, handler| {
        let position = Vector3 {
            x: packet.decode_double()?,
            y: packet.decode_double()?,
            z: packet.decode_double()?,
        };
        let rotation = Vector2 {
            x: packet.decode_float()?,
            y: packet.decode_float()?,
        };
        let flags = packet.read_u8()?;
        let player = handler.player.as_mut().ok_or(RuntimeError::UnexpectedNone)?;
        player.position = position;
        player.rotation = rotation;

        _ = SERVER.sender.send(Data::UpdatePosition {
            player_key: PlayerKey::from(&player),
            position, rotation,
            on_ground: (flags & 0x01) != 0
        });
        Ok(())
    });

    responses.insert(0x1F, |packet, handler| {
        let rotation = Vector2 {
            x: packet.decode_float()?,
            y: packet.decode_float()?,
        };
        let flags = packet.read_u8()?;
        let player = handler.player.as_mut().ok_or(RuntimeError::UnexpectedNone)?;
        player.rotation = rotation;

        _ = SERVER.sender.send(Data::UpdatePosition {
            player_key: PlayerKey::from(&player),
            position: player.position, rotation,
            on_ground: (flags & 0x01) != 0
        });
        Ok(())
    });

    responses.insert(0x20, |packet, handler| {
        let flags = packet.read_u8()?;
        let player = handler.player.as_mut().ok_or(RuntimeError::UnexpectedNone)?;
        _ = SERVER.sender.send(Data::UpdatePosition {
            player_key: PlayerKey::from(&player),
            position: player.position, rotation: player.rotation,
            on_ground: (flags & 0x01) != 0
        });
        Ok(())
    });

    responses
});
