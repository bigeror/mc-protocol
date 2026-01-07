use std::{collections::HashMap, sync::LazyLock};
use parking_lot::Mutex;

use crate::protocol::datatypes::Vector3;

pub type ChunkSectionMap = HashMap<Vector3<i32>, Box<Section>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub block_data: [u8; 4096],
    pub block_count: i32,
    pub palette: Vec<i32>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct World {
    pub chunks: ChunkSectionMap,
}

const DEFAULT_PALETTE: [i32; 2] = [0, 1];

impl World {
    pub fn new() -> Self { Self {
        chunks: HashMap::new(),
    } }
    pub fn replace_block(&mut self, location: Vector3<i32>, to_id: i32) {
        fn compress_location(location: i32) -> i32 {(location as f64 / 16.0).floor() as i32}
        fn get_chunk_location(location: i32, chunk_location: i32) -> i32 {location - (chunk_location * 16)}

        let chunk_coordinates: Vector3<i32> = Vector3 {
            x: compress_location(location.x),
            y: compress_location(location.y) + 4,
            z: compress_location(location.z),
        };
        let chunk_location: Vector3<i32> = Vector3 {
            x: (7 - get_chunk_location(location.x, chunk_coordinates.x)).rem_euclid(16),
            y: get_chunk_location(location.y + 64, chunk_coordinates.y),
            z: get_chunk_location(location.z, chunk_coordinates.z),
        };
        let block_index = chunk_location.y * 256 + chunk_location.z * 16 + chunk_location.x;

        let mut section = match self.chunks.get(&chunk_coordinates) {
            Some(val) => val.clone(),
            None => Self::generate_section(chunk_coordinates)
        };

        let replace_id =
            if section.palette.contains(&to_id) {section.palette.iter()
                .position(|&val| val == to_id).unwrap()}
            else if section.palette.len() < 255 {section.palette.push(to_id); section.palette.len() - 1}
            else {panic!("too high id")} as u8; // TODO: handle more than 2^4 different ids

        section.block_data[block_index as usize] = replace_id;
        _ = self.chunks.insert(chunk_coordinates, section);
    }
    pub fn generate_section(location: Vector3<i32>) -> Box<Section> {
        // simplest possible chunk generation system
        if location.y >= 12 {Box::new(
            Section { block_data: [0; 4096], block_count: 4096, palette: DEFAULT_PALETTE.into() }
        )}
        else {Box::new(Section { block_data: [1; 4096], block_count: 4096, palette: DEFAULT_PALETTE.into() })}
    }
    pub fn get_section(&mut self, location: Vector3<i32>) -> Box<Section> {
        match self.chunks.get(&location) {
            Some(value) => value.clone(),
            None => {
                let section = Self::generate_section(location);
                _ = self.chunks.insert(location, section.clone());
                section
            }
        }
    }
    pub fn get_block(&mut self, location: Vector3<i32>) -> Option<i32> {
        fn compress_location(location: i32) -> i32 {(location as f64 / 16.0).floor() as i32}
        fn get_chunk_location(location: i32, chunk_location: i32) -> i32 {location - (chunk_location * 16)}

        let chunk_coordinates: Vector3<i32> = Vector3 {
            x: compress_location(location.x),
            y: compress_location(location.y) + 4,
            z: compress_location(location.z),
        };
        let  section = match self.chunks.get(&chunk_coordinates) {
            Some(val) => val.clone(),
            None => return None,
        };
        let chunk_location: Vector3<i32> = Vector3 {
            x: (7 - get_chunk_location(location.x, chunk_coordinates.x)).rem_euclid(16),
            y: get_chunk_location(location.y + 64, chunk_coordinates.y),
            z: get_chunk_location(location.z, chunk_coordinates.z),
        };
        let block_index = chunk_location.y * 256 + chunk_location.z * 16 + chunk_location.x;

        Some(section.palette[section.block_data[block_index as usize] as usize])
    }
}

pub static WORLD: LazyLock<Mutex<World>> = LazyLock::new(|| Mutex::new(World::new()));
