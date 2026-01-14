use std::sync::{Arc, LazyLock};

use crate::{concat_buffer, create_packet_collection, datatypes::Packet, protocol::{datatypes::{Vector2, Vector3}, server::world::WORLD}};

create_packet_collection!(PlayClientBound,
    login: |eid: i32| {Ok(concat_buffer!{
        byte 0x2B,
        int eid, //Player entity id
        byte 0, // is hardcore
        byte 1, str "minecraft:overworld", // dimension names
        byte 1, // max players (ignored)
        byte 2, // view distance
        byte 2, // simulation distance
        byte 0, // less debug info
        byte 1, // enable respawn screen
        byte 0, // limited crafting
        byte 0, // dimension type
        str "minecraft:overworld", // dimension name
        long 0, // hashed seed
        byte 1, // default gamemode
        byte 255, // previous gamemode
        byte 0, // debug world
        byte 1, // is flat (visual)
        byte 0, // has death location
        byte 0, // portal cooldown 
        byte 0, // sea level
        byte 0, // enforce secure chat
    }?.concat())},

    player_info_update: |players: Vec<(Arc<str>, Arc<str>, i32)>| {Ok(concat_buffer!{
        byte 0x3F,
        byte 0x01 | 0x08, // Add player & Update listed
        varint players.len() as i32, // example server with only 1 player (1 item in array)
        buf {
            let mut output = Vec::new();
            for player in players { output.extend(
                concat_buffer!(uuid &player.0, str &player.1, byte 0, byte 1)?
            ) }
            output.concat()
        },
    }?.concat())},

    game_event: |id: u8, data: f32| {Ok(concat_buffer!(
        byte 0x22,
        byte id,
        float data,
    )?.concat())},

    set_center_chunk: |x: i32, y: i32| {Ok(concat_buffer!(
        byte 0x57,
        varint x,
        varint y,
    )?.concat())},

    teleport_player: |
        id: i32,
        position: Vector3<f64>,
        motion: Vector3<f64>,
        direction: Vector2<f32>,
        relative: Option<i32>
    | {Ok(concat_buffer!(
        byte 0x41,
        varint id,
        double position.x,
        double position.y,
        double position.z,
        double motion.x,
        double motion.y,
        double motion.z,
        float direction.x,
        float direction.y,
        int match relative {
            Some(val) => val,
            None => 0
        },
    )?.concat())},

    chunk_batch_start: | | {Ok(concat_buffer!(byte 0x0C)?.concat())},
    chunk_batch_finish: |amount: i32| {Ok(concat_buffer!(byte 0x0B, varint amount)?.concat())},
    keepalive: |id: i64| {Ok(concat_buffer!(byte 0x26, long id)?.concat())},

    send_filled_chunk: |position: Vector2<i32>| {
        let mut sections_data = Vec::new();
        for i in 0..24 {
            let mut world = WORLD.lock();
            let section = world.get_section(Vector3 { x: position.x, y: i, z: position.y });
            drop(world);

            sections_data.extend(concat_buffer!(
                ushort section.block_count as u16, // blocks in section
                byte 8, // bits per entry
                varint section.palette.len() as i32,
                buf {
                    let mut palette_data = Vec::new();
                    for j in section.palette {
                        palette_data.extend(Packet::encode_varint(j)?);
                    }
                    palette_data
                }, // palette
                buf {
                    let mut section_data = Vec::new();
                    for j in section.block_data {
                        section_data.extend(vec![j as u8]);
                    }
                    section_data
                }, // block data
                byte 0, byte 0, // biome data
            )?.concat());
        }

        Ok(concat_buffer!(
            byte 0x27,
            int position.x,
            int position.y,
            byte 0, // no heightmap
            varint sections_data.len() as i32, // chunk section data length (in bytes)
            buf sections_data, // chunk sections
            byte 0, // block entities
            byte 0, // empty data
            byte 0, // empty data
            byte 0, // empty data
            byte 0, // empty data
            byte 0, // empty data
            byte 0, // empty data
        )?.concat())
    },

    send_system_message: |message: Vec<u8>, overlay: bool| {Ok(concat_buffer!(
        byte 0x72,
        buf message,
        byte if overlay {1} else {0},
    )?.concat())},

    aknowledge_block_change: |id: i32| {Ok(concat_buffer!(byte 0x04, varint id)?.concat())},
    block_update: |id: i32, position: Vector3<i32>| {Ok(concat_buffer!(byte 0x08, pos position, varint id)?.concat())},

    summon_entity: |
        id: i32,
        uuid: Arc<str>,
        entity_type: i32,
        position: Vector3<f64>,
        rotation: Vector2<f32>,
        data: i32,
        velocity: Vector3<f64>
    | {Ok(concat_buffer!(
        byte 0x01,
        varint id,
        uuid &uuid,
        varint entity_type,
        double position.x,
        double position.y,
        double position.z,
        byte ((rotation.x / 360.0).rem_euclid(1.0) * 256.0).floor() as u8,
        byte ((rotation.y / 360.0).rem_euclid(1.0) * 256.0).floor() as u8,
        byte ((rotation.y / 360.0).rem_euclid(1.0) * 256.0).floor() as u8,
        varint data,
        short (velocity.x * 8000.0) as i16,
        short (velocity.y * 8000.0) as i16,
        short (velocity.z * 8000.0) as i16,
    )?.concat())},

    remove_entity: |ids: Vec<i32>| {
        let mut ids_buffer = Vec::new();
        for id in &ids {ids_buffer.extend(concat_buffer!(varint *id)?)}
        Ok(concat_buffer!(
            byte 0x46,
            varint ids.len() as i32,
            buf ids_buffer.concat(),
        )?.concat()
    )},

    update_attributes: |propeties: Vec<(i32, f64)>| {Ok(concat_buffer!(
        byte 0x7C,
        varint 0,
        varint propeties.len() as i32,
        buf {
            let mut output = Vec::new();
            for propety in propeties {output.extend(
                concat_buffer!(varint propety.0, double propety.1, byte 0)?
            )}
            output.concat()
        },
    )?.concat())},

    update_position: |eid: i32, delta_pos: Vector3<f64>, rotation: Vector2<f32>, on_ground: bool| {Ok(concat_buffer!(
        byte 0x2F,
        varint eid,
        short (delta_pos.x * 4096.0) as i16,
        short (delta_pos.y * 4096.0) as i16,
        short (delta_pos.z * 4096.0) as i16,
        byte ((rotation.x / 360.0) % 1.0 * 256.0) as u8,
        byte ((rotation.y / 360.0) % 1.0 * 256.0) as u8,
        byte on_ground as u8,
    )?.concat())},
);

pub static CLIENT_BOUND_PACKETS: LazyLock<PlayClientBound> = LazyLock::new(PlayClientBound::init);
