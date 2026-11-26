# Design

## Context

- **Loading:** `backend::load_png` decodes into an `RgbaImage`, and `detect(&RgbaImage)` computes the layout.
- **Server cache:** each database is held as `Db { path, image, layout, ids }` behind its own `RwLock` (see the `keep-databases-in-memory` change). The image is kept for `get_image` (a crop) and `write` (`write_field` on the whole image, then saving the whole PNG).
- **What a field write touches.** `backend::write_field(image, layout, id, value)`:
  - looks the field up in `layout.fields` by id;
  - validates the value, then clears and draws only `field.inner`;
  - returns `read_data(image, field)`.
- **What `read_data` and `read_text` read:** the 3×3 mark area above the outer rectangle's top-right corner (`mark_area`), the data area, and pixel (0,0), which is taken as the background color. Nothing else in the image is read.
- **Dependencies:** the `image` crate is built with only the `png` feature.
- **Paint:** MS Paint saves 24-bit, bottom-up BMPs with a 40-byte `BITMAPINFOHEADER`.

## Goals / Non-Goals

**Goals:**
- With a BMP database open, memory holds only the layout.
- Field images and writes do I/O proportional to the field's size.
- Pixels are interpreted the same way for full reads and partial reads.
- No backend API changes beyond the new BMP module and the loader that picks the format.

**Non-Goals:**
- Lower peak memory during the first scan. The image is still decoded in full once for `detect`.
- A persistent layout index.
- Writing palette or compressed BMPs.

## Decisions

### 1. A small BMP module of our own (`backend::bmp`), not the `image` crate's `bmp` feature

`BmpInfo::parse(header bytes)` reads:
- the `BM` signature and `bfOffBits`;
- the header size: 40, 52, 56, 108 or 124 bytes;
- width, and height (a negative height means top-down);
- 1 plane;
- `bpp` of 24 or 32;
- the compression: `BI_RGB` (0), or `BI_BITFIELDS` (3) only when the masks are R=`00FF0000`, G=`0000FF00`, B=`000000FF` and A is `FF000000` or 0.

Anything else gives `Error::UnsupportedBmp { path, reason }`. `Error::Decode` is not used for BMP files, because it wraps an `ImageError`.

From the header it computes:
- `stride = ((width * bpp + 31) / 32) * 4`;
- `row_offset(y) = bfOffBits + stride * (if bottom_up { height - 1 - y } else { y })`.

Alpha is the 4th byte only for 32-bit files with an alpha mask; otherwise it is 255. The 400 Mpx check runs on the header, before any pixel I/O.

- **`load_bmp(path) -> RgbaImage`:** reads the file in row chunks through a `BufReader`, and fills the image with `put_pixel`.
- **`read_rect(file, &info, rect) -> RgbaImage`:** for each row, seeks to `row_offset(y) + x0 * bytes_per_pixel` and reads `width * bytes_per_pixel` bytes.
- **`write_rect(file, &info, x, y, &RgbaImage)`:** the reverse. A 24-bit file drops the alpha byte; a 32-bit file writes 255 when it has no alpha mask.
- **`save_bmp(path, &RgbaImage)`:** writes a 24-bit bottom-up file with a 40-byte header, for tests and `gen_big`.

*Alternative:* the `image` crate's `bmp` feature for full decoding. This was rejected for two reasons. The partial reader would still have to be written by hand, and two decoders could disagree on alpha in 32-bit BI_RGB files, which would show up as a subtle frame-color mismatch.

### 2. The format follows the extension: `load_image` / `analyze_image`
`.bmp` (in any letter case) goes to `load_bmp`; anything else goes to `load_png`. `analyze_png` stays as a thin wrapper, so existing callers keep working. The CLI and the server call `load_image` and `analyze_image`.

### 3. The server's `Db` keeps pixels either in memory or in a file

```
struct Db { layout, ids, background: Rgba<u8>, pixels: Pixels }
enum Pixels { Memory { path, image: RgbaImage }, Bmp { path, info: BmpInfo } }
```

- **Loading a BMP database:** the first use runs `load_bmp`, `detect` and `record_ids`. It saves `image.get_pixel(0,0)` as `background`, then drops the image.
- **`records_page`:** uses `d.background` instead of the image.
- **`get_image`:**
  - `Memory`: crops the image, as now.
  - `Bmp`: opens the file read-only for each request and calls `read_rect(inner)`. The file is not held open, so Paint and other tools can still open the file. Opening it per request costs very little.

### 4. A BMP write works on a small window and calls the existing `write_field`
Let `R` be the outer rectangle joined with the mark area. The window is a new `RgbaImage` of size `(R.width()+1) × (R.height()+1)`:
- pixel (0,0) is set to `d.background`, which is what `read_data` and `read_text` read as the background;
- `R` is filled from the file with `read_rect`, placed at (1,1).

A copy of the field is shifted by `(1 - R.left, 1 - R.top)`. A layout with just that field (same id) is built, and `write_field(&mut window, &mini, id, value)` is called. It validates the value, draws, and returns the stored `FieldData`. Then:
1. Keep the data area's old pixels, which are already in the window copy taken before the write.
2. `write_rect` the new data area to the file, then `sync_data`.
3. If that fails, `write_rect` the old pixels back (best effort) and return 500.
4. On success, set `layout.fields[id-1].data` to the stored value.

Pixel (0,0) of the window never reaches the file, because only the data area is written back.

*Alternative:* add a backend API that writes into a window with an explicit background. This was rejected because `write_field` already gives the right result with the trick above, and changing the backend API is not needed.

### 5. Uploaded alpha is flattened for BMP databases
In `put_image`, for a BMP database the uploaded bitmap's alpha is set to 255 before the value goes to `write_field`. The `FrameColor` check then sees exactly what the file will store. Without this, a pixel of `#BADBA1` with alpha 254 would pass the check but turn into a frame pixel once stored.

### 6. Ids with both extensions
- **`db_path(dir, id)`:**
  - only one of the two files exists: returns its path and kind;
  - both exist: returns `409 AmbiguousDatabase`;
  - neither exists: returns 404.
- **`list_dbs`:** collects the stems of `.png` and `.bmp` files (in any letter case) into a sorted, de-duplicated list.
- **`ApiError`:** gains a `conflict(kind, message)` constructor.

### 7. Durability trade-off for BMP writes
PNG writes keep the temporary file and rename. A BMP write is an in-place overwrite of a few rows, finished with `sync_data`. A crash in the middle of a write can leave that field half-written, but never the rest of the file. This is accepted, because avoiding a full-file rewrite is the reason for supporting BMP.

## Risks / Trade-offs

- [External edits to a `.bmp` can partly show: field images are read from the file, while values and layout come from the first scan] → Stated in the proposal and in the spec. External edits stay unsupported while the server runs.
- [The first scan of `big.bmp` still decodes 1.6 GB] → Accepted; band-wise detection was ruled out for this change.
- [After the scan, the process may not return the memory to the OS] → Large blocks are freed directly on Windows, and the spec scenario checks this by measuring the server's memory after the first request.
- [A BMP from an unusual tool uses a header variant that is not accepted] → It is rejected with a clear `UnsupportedBmp` reason. Variants can be added later.
- [Another tool holds the file locked on Windows when a write happens] → The write fails with 500, the old bytes are restored, and memory stays unchanged.
