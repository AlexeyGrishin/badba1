# Design

## Context

`backend` is currently a placeholder library, and there is no binary crate. The workspace adds members automatically from `crates/*` (see the `cargo-workspace` spec). Requirements are in `specs/image-layout/spec.md` and `specs/cli/spec.md`.

The reference image (the user's `db/example1.png`, copied to `testdata/example1.png` for tests; 539×348, 8-bit RGBA) was analyzed during planning with a throwaway prototype:
- 7190 pixels are exactly `#BADBA1`. They form only 2 groups, because the lines physically join the frames.
- Originally, 4 pixels were `#BADAA1` where the thin line meets two frames, probably from anti-aliased stroke ends blended over the frame. Each left a 1×1 "hole" in a frame that looked like a tiny field. The image author has since fixed them, and the image now has no near-miss pixels. The closest other color is the gray text antialiasing, `#BBBBBB`.
- Background-colored enclosed regions: 5 interiors, all perfect rectangles.
- Small green and red marks sit just outside the top-right corners of three frames, and the text "просто текст" sits outside every frame. Neither is frame color, so both are ignored.

The algorithm below, prototyped with exact color matching on the fixed image, produced exactly the expected 5 fields and 3 links.

## Goals / Non-Goals

**Goals:**
- Detection works on images drawn by hand in MS Paint: any frame and line thickness, curved lines.
- A pure, testable core (`detect(&RgbaImage) -> Layout`) that is separate from file loading.

**Non-Goals:**
- Interpreting field contents (what is drawn in the data area).
- Fields touching or sharing frames with other fields, fields nested inside fields, and rotated rectangles.
- Fast processing of very large images. Simple O(pixels) passes are enough.

## Decisions

**Pipeline**, all on a boolean "frame color" mask built once:
1. **Mask:** a pixel is frame color if its RGBA is exactly (186, 219, 161, 255), as the spec requires.
2. **Interiors:** flood-fill the non-mask pixels, connecting horizontally and vertically only. Keep regions that don't touch the image edge and whose pixel count equals their bounding-box area, i.e. filled rectangles. This rejects circles, rings and interiors containing frame-color dots.
3. **Frames:** for each interior rectangle, grow the outer rectangle one side at a time. A side moves out by 1 while the entire new row or column, corners included, is frame color. Repeat until no side can grow. If any side grew by 0, it isn't a field. The border thickness of each side is how far that side grew.
4. **Strokes:** frame-color pixels not inside any field's frame (outer rectangle minus interior) are grouped with 8-connectivity (diagonals count, matching how thin Paint lines step diagonally).
5. **Links:** a stroke becomes a link when some stroke pixel is 8-adjacent to the frames of ≥2 distinct fields. Its field list is sorted and has no duplicates.
6. **Numbering:** fields are sorted by (outer top, outer left) and numbered from 1. Links are sorted by their field lists and numbered from 1.

*Alternative considered:* finding rectangles by edge/corner detection or a Hough-style line search. Rejected: hand-drawn lines and curves would need heuristics. Starting from the interior ("a hole shaped like a rectangle") is exact and cheap.

*Alternative considered:* growing the frame with an "at least N% frame color" test instead of 100%. Rejected because it would contradict exact matching: a frame is only a frame when every pixel is the frame color.

**Exact color, no tolerance (a product decision).** The frame color is the database's markup, so it has to be unambiguous. A near-miss pixel is a defect in the image that its author should fix, not something the reader should guess about. The color is a single `const` RGBA value. *Alternatives rejected:* a per-channel tolerance, and discarding tiny interiors (which would hide real 1×1 fields).

**Connectivity:** 4-connectivity for interiors and 8 for strokes. With 8-connectivity, interiors could leak out through diagonal gaps in 1-pixel frames. Strokes need 8 because 1-pixel diagonal lines only touch at corners. This also defines "gap" in the spec: pixels that touch only at a corner are not a gap.

**Library API (`backend`):**
- `Rect { left, top, right, bottom }` with inclusive `u32` pixel coordinates, plus `width()` and `height()`.
- `Borders { top, right, bottom, left }`
- `Field { id, outer, inner, borders }` and `Link { id, fields: Vec<usize> }`
- `Layout { width, height, fields, links }`
- `fn detect(image: &image::RgbaImage) -> Layout` is pure and can be tested with images built in memory.
- `fn analyze_png(path: &Path) -> Result<Layout, Error>` loads, converts to RGBA8, then detects.
- `Error` is an enum of `Io { path, source }` and `Decode { path, source }`, with a `Display` that names the path. It's hand-written, to avoid a `thiserror` dependency.

Modules: `lib.rs` (re-exports), `layout.rs` (types), `detect.rs` (algorithm), `load.rs` (PNG loading and errors).

**PNG decoding:** the `image` crate (0.25) with `default-features = false, features = ["png"]`. It converts every PNG color type and bit depth to RGBA8 in one call. *Alternative:* the lower-level `png` crate, which would need hand-written conversion for palette, grayscale and 16-bit images. `image` is pinned in `[workspace.dependencies]`.

**`cli` crate:** a binary named `cli`, using `std::env::args` with no argument-parsing library, since it takes only one argument for now. Swapping in `clap` is easy when subcommands arrive. Exit codes are 0 on success, 1 on file errors and 2 on usage errors. Output format:
```
testdata/example1.png: 539x348, 5 fields, 3 links
Fields:
  F1  outer (101,22)-(137,47) 37x26  border t5 r5 b5 l5  data (106,27)-(132,42) 27x16
  ...
Links:
  L1  F1 - F2
  L2  F1 - F3
  L3  F4 - F5
```
The "no links" case prints `Links:` followed by nothing, which matches the spec.

**Tests:**
- `backend` unit tests draw images in memory with a small test helper (filled rects, lines, circles) for each spec scenario.
- `backend` integration test (`crates/backend/tests/example1.rs`): checks the exact layout of `testdata/example1.png`, found via `CARGO_MANIFEST_DIR/../../testdata/example1.png`. Tests never read `db/`, which is the user's folder for manual testing. `testdata/` sits at the workspace root because both crates' tests use the same image.
- `cli` integration tests run the built binary (`CARGO_BIN_EXE_cli`) and check stdout, stderr and the exit code.

## Risks / Trade-offs

- [A link wider than the frame side it attaches to, starting flush with the frame's corner, fills the whole next row or column, so the frame grows into the line and reports a too-thick border] → Accepted for an experimental DB. The data area is still correct. Can be refined later by checking frame continuity beyond the corners.
- [Fields that touch each other or share a frame side may get wrong borders] → Out of scope (Non-Goals). Draw a gap or a link between fields.
- [Anti-aliasing can leave near-miss pixels (such as `#BADAA1`) where strokes overlap. With exact matching, such a pixel breaks a frame or a line, causing a false tiny field, a missing field or a missing link] → The image author fixes the pixels; drawing frames and lines with smoothing off avoids them. Soft edges over the background are harmless, because the solid core of a stroke still matches. A later CLI command could warn about pixels close to `#BADBA1` to make this easy.
- [Reading-order numbering is by exact top pixel, so a field 1px higher is numbered first even if it looks like it's in the same row] → Matches the spec. It's deterministic, which matters more than looks for now.
- [The `image` crate pulls in several transitive crates] → Only the PNG feature is enabled.
