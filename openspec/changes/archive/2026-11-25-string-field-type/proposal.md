# Proposal

## Why

Most real data is text. badba1 needs a `string` field whose value is written inside the field in a tiny pixel font that is easy to draw by hand in Paint and reliably readable by the backend. The font itself should be data the user can edit, not code.

## What Changes

- New field type **`string`**, marked `.#.` / `#.#` / `#.#` (an upside-down "v", like an A without its bar). Its value is the text drawn in its data area.
- **Background color** is the color of the whole image's top-left pixel. Inside a string field's data area, every pixel of another color is ink, in any color. Letters may differ in color: the reference image has a pink S among blue letters.
- **Letters** are 3 pixels wide and 5 tall, drawn with the glyphs of a bitmap **font**. Only exact matches with one of the font's glyph forms count. A glyph that matches none becomes `�` (U+FFFD).
- **Spaces:** a horizontal gap of G empty columns between two letters means floor(G / 3) spaces. Gaps before the first or after the last letter of a line are ignored.
- **Lines:** letters whose 5-row cells start on the same row form one line. Letters on a different row form another line. Lines are ordered top to bottom, then left to right, and joined with `\n`.
- **The font is data, embedded at build time** from `crates/backend/font/`:
  - `font.png`: 3×5 glyph cells on a grid with a 4-pixel horizontal and 6-pixel vertical pitch. The first row has one glyph per character. Lower rows hold alternate forms of the character in the same column.
  - `chars.txt`: the characters of the first row, in order.
  
  The initial font is the user-supplied image, saved with this change as `font.png`. It covers `A`–`Z`, `0`–`9`, `_ + - *`, and alternate forms for `A E F G K R U 1 2 3 4 6 7 8 9 0`: 56 forms in all, every one distinct. Changing the font means editing these files and rebuilding.
- `cli` shows `string "<text>"` with Rust-style escaping, so a multi-line value prints as `string "LINE1\nLINE2"`.
- A third reference image, `testdata/example3.png`, is a copy of the user's current `db/example1.png`. Its string field at (297,54)–(338,60) reads **`OPENSPEC`**. That one field already uses both forms of E, letters in two colors, and an N drawn in the font's style.

## Capabilities

### New Capabilities
- `strings`: How the text of a `string` field is read: background and ink, letter cells, font matching, spaces, lines, and unknown glyphs.
- `glyph-font`: The format of the editable bitmap font (image grid plus character list) and how alternate forms are declared.

### Modified Capabilities
- `field-types`: Recognizes the upside-down-v shape as the `string` type.
- `cli`: The field listing shows `string "<text>"`.

## Impact

- `crates/backend`: `FieldData::String { color, text }`, a new `font.rs` (parses the embedded font once), and a new `strings.rs` (text reading). Adds data files `crates/backend/font/font.png` and `chars.txt`. No new dependencies, since the `image` crate already decodes PNG.
- `crates/cli`: a new `match` arm and tests.
- New test image `testdata/example3.png` and test `crates/backend/tests/example3.rs`. `example1` and `example2` are unchanged.
