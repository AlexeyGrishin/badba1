# Design

## Context

Field types are read in `types.rs` from the 3×3 mark (top-middle pixel = mark color) and stored as `FieldData`. References were added as a second phase after records. A string's value depends only on its own data area, so it fits the first phase. See proposal.md for motivation and `specs/strings/spec.md` and `specs/glyph-font/spec.md` for the rules.

Inputs checked during planning:
- **The user's font** (saved with this change as `font.png`, 160×11, black on white) has 40 cell columns × 2 rows on a 4/6-pixel pitch, with the separator column and row empty everywhere.
  - Row 0 reads `ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890_+-*`. The last glyph was confirmed as `*` by the user.
  - Row 1 holds alternate forms for `A E F G K R U 1 2 3 4 6 7 8 9 0`.
  - That's 56 forms, all distinct, and every glyph has ink in its left column.
- **The user's current `db/example1.png`** has 11 fields. F8, at (297,54)–(338,60), has a 1-pixel frame, a 40×5 data area and a black `.#.` / `#.#` / `#.#` mark. Its letters start at x = 298, 302, …, 326 on row 55, with 1-column gaps, blue except for a pink S. Matched against the font, they read `OPENSPEC`: the first E is the main form (`##.` middle), and the second is the alternate (`###`).

## Goals / Non-Goals

**Goals:**
- The font is pure data the user edits (`font.png` plus `chars.txt`), with no glyph tables in code.
- Reading text is deterministic and exact, with no fuzzy matching.

**Non-Goals:**
- Lowercase, proportional fonts, glyphs larger than 3×5, or rotated or skewed text.
- Choosing a font at run time. It's embedded at build time, as the user chose.
- Lines whose cells overlap vertically in the same columns.

## Decisions

**Font loading (`font.rs`):**
- `include_bytes!("../font/font.png")` and `include_str!("../font/chars.txt")`, parsed once into a `Font` behind `std::sync::OnceLock`.
- `Font { glyphs: HashMap<u16, char> }`, where a glyph is a 15-bit mask: bit `y*3+x`, rows top to bottom. Exact matching is then a single hash lookup.
- `Font::parse(png: &[u8], chars: &str) -> Result<Font, FontError>` implements the grid rules:
  - background = top-left pixel
  - cell (c, r) at (4c, 6r)
  - row 0 is the main forms, lower rows are alternates
  - empty cells are skipped
  - chars are counted as `chars()` after trimming a trailing `\r\n`/`\n`
  
  It returns errors for a count mismatch and for a shape shared by two characters.
- The built-in font is loaded with `.expect()`, but a unit test (`builtin_font_is_valid`) checks it loads with 40 characters and 56 forms. A broken font edit therefore fails `cargo test`, not just at run time.

*Alternative:* a `build.rs` that generates Rust tables. Rejected because it adds build complexity for no gain: parsing a 160×11 PNG once at run time costs microseconds.

**Text reading (`strings.rs`), `fn read_text(image, inner: Rect, font) -> String`:**
1. Background = `image.get_pixel(0, 0)`. Ink = data-area pixels ≠ background.
2. 8-connected ink components inside `inner`, each with its bounding box.
3. **Line tops:** every component exactly 5 rows tall contributes its top row, collected into a sorted set.
4. **Assign components to lines:** a component whose rows fit inside some line `[top, top+4]` goes to that line. If several lines fit, it goes to the one with the closest top above it. A component that fits no line, such as one taller than 5 rows or a lone `-`, starts its own line at its top row and will mostly read as `�`.
5. **Per line:** build a column-occupancy array from that line's components only. Walk left to right: the next ink column x starts a cell [x, x+2]. The cell's 3×5 mask is taken from that line's ink at rows top..top+4. Each gap of G empty columns between cells gives floor(G/3) spaces.
6. **Match** the mask in the font, or `'\u{FFFD}'` if it isn't found.
7. Order lines by (top, leftmost column), join with `\n`.

*Alternative:* finding lines from horizontal ink projections. Rejected because two lines side by side at different heights (allowed by the spec) would merge.

**Data model and type reading:** `STRING_MARK` (`.#.` / `#.#` / `#.#`) in `types.rs` gives `FieldData::String { color: [u8; 4], text: String }`. The text is computed right away from `field.inner`. `FieldData` currently derives `Copy`, which a `String` breaks:
- `Copy` is removed.
- Uses that relied on it (`.data` read by value in tests, `match *self` in `percent()`) switch to references or `.clone()`.
- `Eq`/`Hash` remain.

**`cli`:** `FieldData::String { text, .. } => format!("string {text:?}")`. Rust's `Debug` quoting escapes `\n` and `"`, and leaves `�` and other printable characters as they are.

**Tests:**
- `font.rs`:
  - The built-in font is valid (40 characters, 56 forms).
  - Both E forms map to `E`.
  - Synthetic tiny fonts produce the count-mismatch and shared-shape errors.
- `strings.rs`: images built by stamping glyphs from the built-in font (a helper takes a char and its form index), one test per spec scenario. These cover:
  - colors and a non-white background
  - 1-column gaps and touching letters
  - two lines, a short glyph inside a line, and lines side by side
  - an empty field
  - both Es, an unknown glyph, and one pixel off
  - gaps of 1/2 vs 3/4/5 vs 6
  - leading and trailing gaps
- `types.rs`: a string mark test.
- `tests/example3.rs`: `OPENSPEC`.
- `cli`: a unit test for `string "OPENSPEC"` and `string "AB\nCD"`.

## Risks / Trade-offs

- [Removing `Copy` from `FieldData` is a breaking API change] → Only in-repo code uses it. The changes are mechanical.
- [A line made only of short glyphs (for example `---`) has no 5-row letter to anchor it] → Its glyphs are misaligned and read as `�`. This is documented as a known limitation. Write at least one full-height character per line.
- [An image whose top-left pixel isn't the paper color, such as a picture touching the corner] → Everything is read relative to that color, as the user specified.
- [The font has no `.` `,` `:` or space glyph] → Those characters can't be written. The user adds glyphs by editing `font.png` and `chars.txt`.
- [Hand-drawn letters that differ by one pixel from every form read as `�`] → Exact matching was chosen. A new style can be added as an alternate form in the font image.
