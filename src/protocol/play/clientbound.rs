use std::sync::{Arc, LazyLock};

use crate::{concat_buffer, create_packet_collection, protocol::{datatypes::{Vector2, Vector3}}};

create_packet_collection!(PlayClientBound,
    login: | | {Ok(concat_buffer!{
        byte 0x2B,
        int 1, //Player entity id
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
    })},

    player_info_update: |uuid: Arc<str>, username: Arc<str> | {Ok(concat_buffer!{
        byte 0x3F,
        byte 0x01 | 0x08, // Add player & Update listed
        varint 1, // example server with only 1 player (1 item in array)
        uuid &uuid,
        str &username,
        byte 0,
        byte 1,
    })},

    game_event: |id: u8, data: f32| {Ok(concat_buffer!(
        byte 0x22,
        byte id,
        float data,
    ))},

    set_center_chunk: |x: i32, y: i32| {Ok(concat_buffer!(
        byte 0x57,
        varint x,
        varint y,
    ))},
    
    teleport_player: |id: i32, position: Vector3<f64>, motion: Vector3<f64>, direction: Vector2<f32>, relative: Option<i32>| {Ok(concat_buffer!(
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
    ))},

    chunk_batch_start: | | {Ok(concat_buffer!(byte 0x0C))},
    chunk_batch_finish: |amount: i32| {Ok(concat_buffer!(byte 0x0B, varint amount))},
    keepalive: |id: i64| {Ok(concat_buffer!(byte 0x26, long id))},

    send_filled_chunk: |position: Vector2<i32>| {
        let mut sections_data = Vec::new();
        for i in 0..24 {
            sections_data.extend(concat_buffer!(
                ushort 4096,
                byte 0,
                varint if i >= 12 {0} else {1},
                byte 0, byte 0,
            ));
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
    ))},

    send_system_message: |message: Vec<u8>, overlay: bool| {Ok(concat_buffer!(
        byte 0x72,
        buf message,
        byte if overlay {1} else {0},
    ))},

    aknowledge_block_change: |id: i32| {Ok(concat_buffer!(byte 0x04, varint id))},
    block_update: |id: i32, position: Vector3<i32>| {Ok(concat_buffer!(byte 0x08, pos position, varint id))},
);

pub static CLIENT_BOUND_PACKETS: LazyLock<PlayClientBound> = LazyLock::new(PlayClientBound::init);
