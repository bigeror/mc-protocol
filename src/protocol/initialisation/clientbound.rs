use std::{collections::HashMap, ops::Deref, sync::{Arc, LazyLock}};

use crate::{create_packet_collection, concat_buffer, protocol::datatypes::PacketCreateError};
use crab_nbt::nbt;
use json::{object, stringify};

pub struct ClientBoundPackets {
    pub status: StatusClientBound,
    pub connect: ConnectClientBound,
}

create_packet_collection!(StatusClientBound,
    status_response: | | {
        let status_text = &stringify(object! { version: {
                name: "§d§lskye fan server! 1.21.8§r",
                protocol: 772
            },
            players: { max: -1, online: 0, sample: [] },
            description: { text: "§d§lSkye fan little server written in rust! :D" },
            enforcesSecureChat: false
        });
        Ok(concat_buffer!{
            byte 0,
            str status_text,
        })
    },
    ping_response: |packet: &Vec<u8>| {Ok(concat_buffer!{ byte 1, buf packet[1..9].to_vec() })},
);

create_packet_collection!(ConnectClientBound,
    login_success: |username: Arc<str>, uuid: Arc<str>| {Ok(concat_buffer!{
        byte 2,
        uuid &uuid,
        str &username,
        byte 0,
    })},
    plugin_message: | | {Ok(concat_buffer!{
        byte 1,
        str "minecraft:brand",
        str "§d§lskye fan silly server! :D§r",
    })},
    send_datapacks: | | {Ok(concat_buffer!{
        byte 0x0E,
        byte 1,
        str "minecraft",
        str "core",
        str "1.21.8",
    })},
    registry_data: |id: Arc<str>, values: Arc<[Arc<str>]>| {Ok(concat_buffer!{
        byte 0x07,
        str &id,
        varint values.len() as i32,
        buf {
            let mut output: Vec<u8> = vec![];
            for _item in values.to_vec() {
                output.extend(concat_buffer!(str &_item, byte 0));
            }
            output
        },
    })},
    registry_data_filled: |id: Arc<str>, values: HashMap<Arc<str>, Vec<u8>>| {Ok(concat_buffer!{
        byte 0x07,
        str &id,
        varint values.len() as i32,
        buf {
            let mut output: Vec<u8> = vec![];
            for (_item, _value) in values.iter() {
                output.extend(concat_buffer!(str &_item, byte 1, buf _value.deref().to_vec()));
            }
            output
        },
    })},
    configuration_finish: | | {Ok(concat_buffer!{byte 0x03})},
);

pub static CLIENT_BOUND_PACKETS: LazyLock<ClientBoundPackets> = LazyLock::new(|| ClientBoundPackets {
    status: StatusClientBound::init(),
    connect: ConnectClientBound::init(),
});

pub fn default_registry_data() -> Result<Vec<u8>, PacketCreateError> {
    let mut dimension_type: HashMap<Arc<str>, Vec<u8>> = HashMap::new();
    let n64 = -64;
    dimension_type.insert("minecraft:overworld".into(), nbt!("", {
        "fixed_time": 6000,
        "has_skylight": true,
        "has_ceiling": false,
        "ultrawarm": false,
        "natural": true,
        "coordinate_scale": 1.0,
        "bed_works": true,
        "respawn_anchor_works": true,
        "min_y": n64,
        "height": 256,
        "logical_height": 256,
        "infiniburn": "#minecraft:infiniburn_overworld",
        "effects": "minecraft:overworld",
        "ambient_light": 1.0,
        "piglin_safe": true,
        "has_raids": true,
        "monster_spawn_light_level": 0,
        "monster_spawn_block_light_limit": 0,
    }).write_unnamed().to_vec());
    Ok([
        (CLIENT_BOUND_PACKETS.connect.registry_data_filled)("minecraft:dimension_type".into(), dimension_type)?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:cat_variant".into(), ["minecraft:british_shorthair".into()].into())?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:chicken_variant".into(), ["minecraft:cold".into()].into())?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:frog_variant".into(), ["minecraft:cold".into()].into())?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:cow_variant".into(), ["minecraft:cold".into()].into())?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:pig_variant".into(), ["minecraft:cold".into()].into())?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:painting_variant".into(), ["minecraft:fire".into()].into())?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:wolf_sound_variant".into(), ["minecraft:angry".into()].into())?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:wolf_variant".into(), ["minecraft:chestnut".into()].into())?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:worldgen/biome".into(), ["minecraft:plains".into()].into())?,
        (CLIENT_BOUND_PACKETS.connect.registry_data)("minecraft:damage_type".into(), [
            "in_fire".into(),
            "campfire".into(),
            "lightning_bolt".into(),
            "on_fire".into(),
            "lava".into(),
            "hot_floor".into(),
            "in_wall".into(),
            "cramming".into(),
            "drown".into(),
            "starve".into(),
            "cactus".into(),
            "fall".into(),
            "ender_pearl".into(),
            "fly_into_wall".into(),
            "out_of_world".into(),
            "generic".into(),
            "magic".into(),
            "wither".into(),
            "dragon_breath".into(),
            "dry_out".into(),
            "sweet_berry_bush".into(),
            "freeze".into(),
            "stalagmite".into(),
            "outside_border".into(),
            "generic_kill".into(),
            "player_attack".into(),
        ].into())?,
    ].concat())
}

