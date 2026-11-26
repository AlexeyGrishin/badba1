# Design

## Context

Measured today: `cli db/big.png` fails within 80 ms with `Memory limit exceeded`. The cause is the `image` crate's default `Limits` (`max_alloc` = 512 MiB). The image is 10000×40000, 8-bit RGB, which is 400 Mpx: 1.2 GB decoded as RGB and 1.6 GB as RGBA.

Past the decoder, the detection pipeline (`detect.rs`, then `references.rs`) has per-pixel costs that become severe at 400 Mpx:

| Where | Buffer | Bytes/px | At 400 Mpx |
|---|---|---|---|
| `detect::Grid` | `mask: Vec<bool>` | 1 | 0.4 GB |
| `Grid::regions` | `seen: Vec<bool>` | 1 | 0.4 GB |
| `Grid::regions` | `region: Vec<(u32,u32)>`, which holds the whole connected background | 8 | up to 3.2 GB |
| `find_links` | `owner: Vec<Option<usize>>` | 16 | 6.4 GB |
| `find_links` | `seen: Vec<bool>` | 1 | 0.4 GB |
| `references::Grid` | `inside: Vec<bool>` | 1 | 0.4 GB |
| `references::Grid` | `frame_owner: Vec<Option<usize>>` | 16 | 6.4 GB |
| `references::Grid::trace` | `seen: Vec<bool>`, allocated and zeroed **once per ref field** | 1 | 0.4 GB × 9000 refs ≈ 3.6 TB of memset |

The server decodes through its own `image::open` in `crates/server/src/api.rs::load`, which has the same 512 MiB cap. Each request loads the whole file again.

The machine has 32 GB of RAM. `db/` is gitignored, so `big.png` is regenerated with `cargo run --release -p backend --example gen_big`. That command's defaults are 10000×40000 and 9000 records of 8 fields, one of them a `ref`.

## Goals / Non-Goals

**Goals:**
- Analyze a 400 Mpx image with peak memory of about 5 GB, and in under a minute in a release build.
- Have one loader that both the CLI and the server use, so they cannot drift apart again.
- Keep detection output identical to today's for every image.

**Non-Goals:**
- Streaming or tiled decoding. The whole image is still held in memory as RGBA8.
- Caching layouts between REST requests, or reducing per-request latency.
- Images over 400 Mpx, and any setting to change the maximum.

## Decisions

### 1. Limit by pixel count, checked from the PNG header
`backend` exports `pub const MAX_PIXELS: u64 = 400_000_000`. The loader reads the file's bytes and calls `ImageReader::with_format(Cursor::new(&bytes), Png).into_dimensions()`, which parses only the IHDR chunk. It returns a new `Error::TooLarge { path, width, height }` if `width as u64 * height as u64 > MAX_PIXELS`. Only then does it decode, with `Limits { max_alloc: Some(MAX_PIXELS * 8), .. }`. The 8 bytes per pixel covers 16-bit RGBA, the widest PNG layout. The width and height limits are left unset.
- *Alternative:* set only `max_image_width` and `max_image_height`. This was rejected because it would cap each side separately instead of the total, and the spec allows any shape up to the pixel count.
- *Alternative:* set `Limits::no_limits()`. This was rejected because a crafted header could then ask for a buffer of terabytes.

Error text: `cannot load <path>: image is too large: 20001x20000 (400020000 pixels), maximum is 400000000 pixels`.

### 2. One public loader: `backend::load_png(path) -> Result<RgbaImage, Error>`
`analyze_png` becomes `load_png` followed by `detect`. The loader converts with `DynamicImage::into_rgba8()` instead of `to_rgba8()`, which avoids a second 1.6 GB copy when the PNG is already RGBA8. It drops the file bytes before returning. The server's `load()` calls `backend::load_png` and maps the errors: `TooLarge` becomes `ApiError { 500, "ImageTooLarge", .. }`, and anything else stays `Internal`.

### 3. `regions` reports a summary, not a pixel list
The only caller, `find_fields`, uses just the bounding box and pixel count (`filled_rect`). So `regions` will pass `visit` a `(Rect, count)`, built up as the BFS runs. This removes the up-to-3.2 GB `region` vector. The BFS queue only holds the frontier, which is small for these images.

### 4. One shared per-pixel field map with 4-byte entries
`find_links` and `references::Grid` both build a per-pixel "which field's frame is this" array, and the two are the same. Build it once in `detect()` as a `FieldMap`: `owner: Vec<u32>`, where 0 means none and otherwise holds the field id, plus `inside: Vec<bool>`. Pass it to both `find_links` and `references::resolve`. Field ids are already ≥ 1, so 0 is free to mean "none". This takes memory from 2 × 6.4 GB down to 1.6 GB + 0.4 GB.
- *Alternative:* a spatial index (a bucket grid of field rects) with no per-pixel array. That saves another 1.6 GB, but it changes the lookup logic in two modules. It was rejected for now because the 5 GB goal is met without it.

### 5. `trace` uses a set local to each ref, not a full-image buffer
A ref line has at most a few thousand pixels, so `trace` keeps visited pixels in a `HashSet<(u32, u32)>`. This replaces the 0.4 GB `seen` buffer that was allocated for each ref field.

### 6. Free buffers stage by stage
The `seen` buffers in `regions` and `find_links` are dropped as soon as each pass ends. `detect::Grid.mask` is dropped before references are resolved. Estimated peak is RGBA 1.6 GB + mask 0.4 + owner 1.6 + inside 0.4 + one `seen` 0.4 ≈ **4.4 GB**. The RGB→RGBA conversion briefly holds both images, 2.8 GB, before any detection buffers exist.

### 7. Large-image regression test is opt-in
`crates/backend/tests/large_image.rs` holds an `#[ignore]` test that analyzes `db/big.png`, or the path in `BADBA1_BIG`. It prints a skip message and passes if the file is missing. It asserts 10000×40000, 72000 fields and 9000 records. Run it with `cargo test --release -p backend -- --ignored`. The too-large cases use a header-only PNG: the signature plus an IHDR chunk with a correct CRC that declares 20001×20000. Since rejection happens before decoding, no huge buffer is needed. `gen_big.rs` is committed as part of this change.

## Risks / Trade-offs

- [Memory still scales with image size, about 11 bytes per pixel] → The goal is set for this machine's 32 GB. The spatial-index alternative is recorded in Decision 4 in case it is needed.
- [The REST API takes seconds per request on `big.png`, because each request decodes and analyzes the whole image, and a write also re-encodes the PNG] → Accepted for an experimental database. Layout caching is listed as a non-goal.
- [Header-only test PNG is not a "valid PNG" in the strict sense] → The behavior under test happens before the data is read, so the result is the same. The maximum-size success path is covered by the real `big.png` test.
- [Refactors in `detect`/`references` could change results] → All existing unit and integration tests must pass unchanged. Comparing `cli` output on `testdata/*.png` before and after confirms that the output is identical.
- [Debug builds are much slower on 400 Mpx] → The large test is documented as release-only, and is ignored by default.
