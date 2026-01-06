use crate::protocol::{datatypes::Vector3, server::mapping::BlockType};


pub fn get_block_id(
    pos: Vector3<i32>,
    offset: Vector3<f32>,
    face: Vector3<i32>,
    block: &BlockType
) -> (i32, Vector3<i32>) {
    (match block {
        BlockType::Block(id) => *id,
        BlockType::Slab(t0, t1, t2) => {
            if face.y == 1 {*t0}
            else if face.y == -1 {*t1}
            else if offset.y <= 0.5 {*t0}
            else {*t1}
        },
    }, pos + face)
}
