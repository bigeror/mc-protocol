use crate::{datatypes::{DatatypeError, Packet}, protocol::{datatypes::Vector3, server::world::datatypes::Section}};

pub struct World {}

impl World {
    pub fn replace_block(location: Vector3<i32>, to_id: i32) {
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
        let block_index = (chunk_location.y * 256 + chunk_location.z * 16 + chunk_location.x) as usize;

        let section = Section::get_section(chunk_coordinates);

        section.data[block_index] = to_id;
        _ = section.mmap.flush();
    }

    pub fn get_block(location: Vector3<i32>) -> i32 {
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
        let block_index = (chunk_location.y * 256 + chunk_location.z * 16 + chunk_location.x) as usize;

        Section::get_section(chunk_coordinates).data[block_index].clone()
    }

    pub fn get_section(location: Vector3<i32>) -> Result<Vec<u8>, DatatypeError> {
        let section = Section::get_section(location);
        let mut output: Vec<u8> = vec![];
        for item in section.data.clone().iter() {
            output.extend(Packet::encode_varint(*item)?);
        }
        Ok(output)
    }
}
