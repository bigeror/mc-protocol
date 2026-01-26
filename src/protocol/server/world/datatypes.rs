use std::{fs::{File, OpenOptions, create_dir}, io::Write, path::Path};

use memmap2::MmapMut;

use crate::protocol::datatypes::Vector3;

pub struct Section<'a> {
    pub bits_per_entry: u8,
    pub section_mapping: &'a mut [u8],
    pub palette: &'a [i32],
    pub file: File,
    pub mmap: MmapMut,
}

pub fn take_slice<const LENGTH: usize>(slice: &[u8], offset: usize ) -> [u8; LENGTH] {
    assert!(LENGTH + offset <= slice.len());
    unsafe { *(slice.as_ptr().add(offset) as *const [u8; LENGTH]) }
}

impl<'a> Section<'a> {
    /// takes an open correct file, converts it to Chunk and maps section data
    pub fn from_filled_file(file: File) -> Self {
        let buffer_length = file.metadata().expect(&format!("unable to get the length of {:?}", file)).len();

        // assume the file structure is correct & no modification happen during runtime
        let mut mmap = unsafe { MmapMut::map_mut(&file).expect(&format!("unable to map file {:?}", file)) };
        // unsafely cast mmap to mutable slice reference
        let slice = unsafe { std::slice::from_raw_parts_mut(mmap.as_mut_ptr(), buffer_length as usize) };

        let mut index: usize = 0;

        // 0 for 1 block only
        let bpe = slice[0];
        let section_mapping = unsafe { &mut *(std::slice::from_raw_parts_mut(
            slice.as_mut_ptr().add(index + 1), 4096 * bpe as usize
        )) };
        index += 4096 * bpe as usize + 1;

        let palette_length = i32::from_be_bytes(take_slice::<4>(slice, index)) as usize;
        let palette = unsafe { std::slice::from_raw_parts::<i32>(
            slice.as_ptr().add(index + 4).cast::<i32>(), palette_length
        ) };

        Self { section_mapping, palette, file, mmap, bits_per_entry: bpe }
    }

    /// takes an open empty file, converts it to Chunk and maps section data
    pub fn from_new_file(mut file: File, position: Vector3<i32>) -> Self {
        let data: [u8; 9] = [
            0, // 0 bpe, skip section data
            0, 0, 0, 1, // 1 palette length (integer)
            0, 0, 0, (position.y < 12) as u8, // block filling
        ];

        let mut buf = data.as_slice();
        _ = file.write_all(&mut buf).or_else(
            |err| -> Result<(), ()> {panic!("error writing to file: {}", err)}
        );
        Self::from_filled_file(file)
    }

    pub fn set_block (&mut self, position: Vector3<i32>, id: i32) {}

    pub fn get_chunk(position: Vector3<i32>) -> Self {
        let folder_path = Path::new("./world");
        if !folder_path.exists() {create_dir(folder_path).expect("unable to create ./world folder")}
        let path = folder_path.join(format!("{}_{}_{}.dat", position.x, position.y, position.z));

        if !path.exists() {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(path.clone()).expect(&format!("couldn't create file {}", path.display()));
            return Self::from_new_file(file, position);
        }

        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(path.clone()).expect(&format!("couldn't create file {}", path.display()));
        Self::from_filled_file(file)
    }
}
