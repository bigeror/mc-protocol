use std::{collections::HashMap, sync::LazyLock};
use crab_nbt::Nbt;

#[derive(Debug)]
pub struct Mapping {
    pub item_to_block: HashMap<i32, i32>,
    pub block_to_item: HashMap<i32, i32>,
}

pub static MAP: LazyLock<Mapping> = LazyLock::new(Mapping::new);

impl Mapping {
    const DEFAULT_RAW: &[u8; 22894] = include_bytes!("../../constants/block_mapping.dat");

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
            let item = compound.get("item")
                .expect(&format!("imported data is incorrect: compound #{} doesn't contain item field", index))
                .extract_int()
                .expect(&format!("imported data is incorrect: item field in compound #{} isn't an integer", index));

            output.item_to_block.insert(item, block);
            output.block_to_item.insert(block, item);
            index += 1;
        }

        output
    }
}


