# Proposal

## Why

`db/big.png` (10000×40000 pixels, 77 MB on disk, made by `crates/backend/examples/gen_big.rs`) cannot be opened: `cli db/big.png` fails at once with `cannot decode db/big.png as PNG: Memory limit exceeded`. The `image` crate refuses any decode buffer over its default 512 MiB cap, and this image needs 1.2 GB as RGB and 1.6 GB as RGBA. Removing the cap alone is not enough. Detection keeps several buffers with one entry per pixel, and some of them are 16 bytes per pixel. One buffer is also allocated again for every `ref` field. At 400 million pixels that comes to tens of GB of memory and hours of work. We want databases of this size to work, and to make this size the documented maximum.

## What Changes

- Define a maximum database image size of **400,000,000 pixels** (width × height), which is exactly the size of `big.png` (10000×40000). Any shape within that pixel count is allowed.
- Raise the PNG decoder limits so every image up to the maximum decodes, including 16-bit PNGs. Check an image's dimensions from its PNG header before decoding it, and reject anything larger with a clear "image too large" error that gives its size and the maximum. A too-large image is rejected without allocating its pixel buffer.
- Use one shared, size-checked PNG loader in the backend for both the CLI and the server. The server's own `image::open` call is replaced.
- Make field, link and reference detection fit images of the maximum size in bounded memory and time:
  - Per-pixel field ids become 4 bytes per pixel instead of 16.
  - Region scanning tracks a bounding box and pixel count instead of storing every pixel of the huge background region.
  - Reference tracing no longer allocates a full-image `seen` buffer for each `ref` field.
  - Intermediate buffers are freed before the next stage starts.
- Behavior on images that already work stays the same. Detection results are unchanged.
- Add a regression check that generates a maximum-size database with `gen_big` and analyzes it. It is marked ignored or release-only, so it does not slow down the normal test run.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `image-layout`: a new "Maximum image size" requirement. Images of up to 400,000,000 pixels must be read and analyzed. Larger images are rejected with a "too large" error before decoding.
- `cli`: "File errors" also covers images that are too large.
- `rest-api`: "Error responses" gains an `ImageTooLarge` error for a database whose image is over the maximum.

## Impact

- `crates/backend/src/load.rs`: new size check and decoder limits, plus a public loader that returns the RGBA image.
- `crates/backend/src/detect.rs` and `crates/backend/src/references.rs`: memory and time reductions in `Grid`, `regions`, `find_links` and `trace`.
- `crates/server/src/api.rs`: `load()` uses the backend loader and maps the too-large error to JSON.
- `crates/backend/examples/gen_big.rs` (currently untracked) is committed and used by the large-image check.
- No new dependencies. `image` 0.25 already provides `ImageReader::limits`.
- Not in scope: making the REST API fast on huge databases. Each request still decodes and analyzes the whole image, which will take seconds at this size. Also not in scope: request body limits for field image uploads, and how the web UI shows huge images.
