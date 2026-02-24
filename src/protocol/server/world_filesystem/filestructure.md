# file structure of the world

## world -> x-y-z.world

In the "world" folder files with names "integer-integer-integer.world" (where integers are base 16) stored. They are being stored runtime as opened files, and when need to read / modify the file they are being mapped using memmap2 for minimal RAM usage possible.

## structure of the section file

- bits per entry (bpe):
  1 byte telling how many bits per one entry is in paletted container. The palette might store 2^bpe amount of entries maximum before it needs to resize. If bpe is 0 data field is being skipped and treated as being 0 too.
- data (if bpe isn't 0):
  Container itself, 4096 (16x16x16) entries with length of bpe, storing the index of the corresponding block id in palette. Being treated as 0 if bpe is 0.
- palette:
  Dynamic list of integers storing maximum 2^bpe amount of elements.

Maximum file size = 1 + (4096 * 12 / 8) + (4096 * 4) = 22529 bytes.
Files size of section with 1 block type = 1 + 0 (omitted) + 1 * 4 = 5 bytes.

## functions on the section file (unsafe, underlying functionality)

- get_mutable: get the file mutable copy (to change a block).

- get_entry: gets the entry index 0-4095 and outputs corresponding palette index bits.
- set_entry: gets the entry index 0-4095 and the palette index and sets the underlying palette entry bits.

- add_entry: add a palette entry (if palette entry amount exceeds 2^bpe calls resize_section and tries again), expected to be called automatically.
- remove_entry: removes an entry and iterates through each field to resize it (if 2^bpe is larger than new palette length calls resize_section).

- resize_section: creates a buffer with corresponding bpe (and data length), maps all the entries from previous data to a new one with corresponding length, copies the palette to a new one and replaces the existing file with the buffer.

## safe functions

- set_block: takes a position and an id and places the block there - checks whether the id is present in palette, and if not - runs add_entry on it. Manages safety.
- get_immutable: get the file immutable copy (to send as a new chunk).
