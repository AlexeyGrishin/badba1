# Proposal

## Why

A PNG database must be decoded as a whole, so the server keeps the decoded image in memory: 1.6 GB for a 10000×40000 database such as `big.png`. Every write re-encodes the whole file, which takes seconds at that size. An uncompressed BMP stores its pixels as fixed-size rows at known offsets. Any rectangle can therefore be read, or overwritten in place, without touching the rest of the file. Supporting BMP databases lets the server keep only the layout in memory, and read or write just the bytes a request needs. BMP is also the format MS Paint opens and saves natively.

## What Changes

- **BMP files are read.** The backend reads uncompressed BMP files with 24-bit pixels, or 32-bit pixels using the standard masks. It accepts both bottom-up and top-down row order, and the header versions Paint and common tools write. Compressed, palette and other bit-depth BMPs are rejected with an "unsupported BMP" error that names the format. The 400,000,000-pixel maximum applies, and the size is checked from the header.
- **BMP rectangles can be read and written.** The backend can read any rectangle of a BMP file, and overwrite one in place, without reading the rest of the file. It can also write an image out as a 24-bit BMP, for tests and for converting PNG databases.
- **The CLI accepts `.bmp`.** The format is chosen from the file extension (`.bmp` or `.png`).
- **The server serves `.bmp` databases next to `.png` ones.**
  - **Ids:** a database's id is its file name without the extension. If both `<id>.png` and `<id>.bmp` exist, the id is listed once, and every request to it gets a 409 `AmbiguousDatabase` error.
  - **First use:** a BMP database is scanned once, the first time a request names it. The full image is decoded only for detection and dropped afterwards. Only the layout and the background color stay in memory.
  - **Field images:** reading a field's image reads only the rows of that field's data area from the file.
  - **Writes:** a write cuts a small window around the field from the file (its frame, type mark and data area), writes the value into that window, and overwrites only the data-area bytes in the file. The file size and every other byte stay the same. If the in-place write fails, the previous bytes are written back. A 24-bit BMP cannot store transparency, so uploaded pixels are made fully opaque before the frame-color check.
- **BREAKING (behavior), BMP only:** external edits to a BMP database file while the server runs are still unsupported. Unlike PNG, though, they can partly show through: image data is read from the file, while field values and the layout come from the scan made at first use. PNG databases keep their current behavior.
- **Not in scope:**
  - A saved layout index, or scanning the image in bands. The first use still decodes the whole image once, so peak memory during that scan is unchanged.
  - Converting existing PNG databases automatically.
  - Writing BMP formats other than 24-bit.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `image-layout`: a new "Read BMP images" requirement covers the supported formats, unsupported ones, a layout identical to the same image saved as PNG, and the maximum size.
- `cli`: "Show layout of an image" accepts `.bmp` as well as `.png` files.
- `rest-api`:
  - "Database ids" covers `.bmp` files and the ambiguous-id error.
  - "Databases held in memory" limits its "not seen until restart" rule to PNG.
  - A new "BMP databases keep pixels on disk" requirement says what is held in memory and that writes patch the file in place.

## Impact

- **`crates/backend/src/bmp.rs` (new):**
  - Header parsing and validation.
  - A full read into an `RgbaImage` (used for detection).
  - `read_rect` and `write_rect` on an open file.
  - `save_bmp` (writes 24-bit, bottom-up).
  - No new dependencies, and the `image` crate's `bmp` feature is not used: one reader means full and partial reads interpret pixels the same way.
- **`crates/backend/src/load.rs`:** a `load_image` / `analyze_image` pair that chooses PNG or BMP from the file extension. `analyze_png` stays.
- **`crates/cli/src/main.rs`:** calls `analyze_image`.
- **`crates/server/src/api.rs`:**
  - `Db` holds either in-memory PNG pixels or a BMP file reference plus its background color.
  - `db_path` and `list_dbs` handle both extensions.
  - `get_image` and `write` branch on the database kind.
- **`crates/backend/examples/gen_big.rs`:** writes a BMP when the output path ends in `.bmp`, so `big.bmp` can be generated.
- **Memory with a BMP database:** the layout only, a few MB for 72000 fields, instead of 1.6 GB. Reading a field's image or writing a field touches a few KB of the file.
