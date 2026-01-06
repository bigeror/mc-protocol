use std::{collections::HashMap, sync::LazyLock};
use crab_nbt::Nbt;

#[derive(Debug)]
pub struct Mapping {
    pub item_to_block: HashMap<i32, BlockType>,
    pub block_to_item: HashMap<i32, i32>,
}

#[derive(Debug)]
pub enum BlockType {
    Block(i32),
    Slab(i32,i32,i32),
}

pub static MAP: LazyLock<Mapping> = LazyLock::new(Mapping::new);

impl Mapping {
    const DEFAULT_RAW: &[u8; 24320] = include_bytes!("../../constants/block_mapping.dat");

    pub fn new() -> Self {
        let mut slice = &Self::DEFAULT_RAW.clone()[..];
        let mut output = Self {
            item_to_block: HashMap::new(),
            block_to_item: HashMap::new(),
        };

        let value = Nbt::read(&mut slice).expect("imported data isn't valid NBT");
        let list = value.root_tag.get("map")
            .expect("imported data is incorrect: no \"map\" compound")
            .extract_list()
            .expect("imported data is incorrect: map is not list type");

        let mut index = 0;
        for value in list {
            let compound = value.extract_compound()
                .expect(&format!("imported data is incorrect: object #{} in map isn't compound", index));
            let block = compound.get("block")
                .expect(&format!("imported data is incorrect: compound #{} doesn't contain block field", index))
                .extract_int()
                .expect(&format!("imported data is incorrect: block field in compound #{} isn't an integer", index));

            let output_block: BlockType;
            if let Some(slab_offset) = compound.get("slab") {
                let offsets = slab_offset.extract_int_array()
                    .expect("imported data is incorrect: slab field isn't Int array.");
                output_block = BlockType::Slab(block + offsets[0], block + offsets[1], block + offsets[2]);
            }
            else {output_block = BlockType::Block(block)}

            let item = compound.get("item")
                .expect(&format!("imported data is incorrect: compound #{} doesn't contain item field", index))
                .extract_int()
                .expect(&format!("imported data is incorrect: item field in compound #{} isn't an integer", index));

            output.item_to_block.insert(item, output_block);
            output.block_to_item.insert(block, item);
            index += 1;
        }

        output
    }
}


