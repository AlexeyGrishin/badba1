# Design

## Context

The backend reads only. `detect(&RgbaImage) -> Layout` returns fields with their outer and inner rectangles and their `FieldData`. `types::read_data(image, field)` reads one field's value (references need records and are resolved separately). The built-in glyph font is available through `font::builtin()`. See proposal.md for motivation and `specs/field-writes/spec.md` for the rules.

Field sizes in `testdata/example3.png`, used as concrete examples in the spec:
- **F2** `counter`, 27×16. At 1×1 with 1-pixel gaps it holds 14 × 8 = 112 squares.
- **F3** `float`, 28×19 = 532 pixels.
- **F6/F7** `bool`.
- **F8** `string`, 40×5: one line of at most 10 letters.
- **F1** `image`, 33×20.
- **F5/F11** `ref`.

## Goals / Non-Goals

**Goals:**
- A small, synchronous, in-memory API that later front-ends (cli, REST) can wrap.
- Writes that always read back as intended, with the returned value proving it.

**Non-Goals:**
- Resizing or moving fields, writing references, or any file I/O. The caller loads and saves the PNG.
- Choosing letter forms or colors for strings. Text uses the main form and the mark color.

## Decisions

**API (`write.rs`):**
```rust
pub enum WriteValue { Counter(u64), Float(f64), Bool(bool), String(String), Image(RgbaImage) }

pub enum WriteError {
    NoSuchField(usize),
    NotWritable { field: usize, type_name: &'static str },        // ref
    TypeMismatch { field: usize, field_type: &'static str, value_type: &'static str },
    FieldOverflow { field: usize, reason: String },
    OutOfRange { field: usize, value: f64 },                      // float outside 0..=100 or not finite
    UnknownCharacter { field: usize, character: char },
    FrameColor { field: usize, x: u32, y: u32 },                  // pixel in the given bitmap
}

pub fn write_field(image: &mut RgbaImage, layout: &Layout, field: usize, value: WriteValue)
    -> Result<FieldData, WriteError>;
```
- `layout` is the caller's result of `detect(image)`.
- `WriteError` implements `Display`/`Error`. The names follow the user's wording (FieldOverflow).

**Validate, then draw.** Each type first computes a complete plan (which pixels get which color) and returns any error before touching the image. Only then does it clear `inner` to the background (`image[(0,0)]`) and paint. This is how "failed writes change nothing" is guaranteed, without copying the image.

**Read-back** reuses `types::read_data(image, field)` on the modified image, so the returned value comes from exactly the code that analysis uses. `ref` is never written, so no record context is needed.

**Per-type plans:**
- *Counter:* for N = 0, clear only. Otherwise find the largest `s ≥ 1` with `((w+1)/(s+1)) * ((h+1)/(s+1)) ≥ N`. If `s = 1` doesn't fit, it's FieldOverflow. Square `i` goes at column `i % cols` and row `i / cols`, with its top-left at `(c*(s+1), r*(s+1))` and size `s×s`. The 1-pixel gaps keep squares apart under the reader's 4-connectivity.
- *Float:* reject a non-finite N or one outside `0..=100`. `k = (N * area / 100).round()` (half away from zero). Fill the first `k` pixels in column-major order: `x = i / h`, `y = i % h`.
- *Bool:* true fills `inner` with the mark color, false clears only.
- *String:* a cursor `(x, y)` starts at (0,0).
  - `'\n'` → `x = 0, y += 6`.
  - `' '` → `x += 3`.
  - A font character uses the main form from the font (`forms()` gives the main form first). It needs `x + 3 ≤ w` and `y + 5 ≤ h`, or the write fails with FieldOverflow. The glyph is stamped, then `x += 4`.
  - A character not in the font gives UnknownCharacter.
  
  This adds a character → main-form lookup to `Font` (`main_form(ch)`).
  
  Every N spaces between letters leave `1 + 3N` empty columns, which reads back as N spaces (floor((1+3N)/3) = N). Leading or trailing spaces and empty lines are drawn as nothing, so they don't survive a read. The returned read-back value shows the text that was actually stored.
- *Image:* the bitmap must be no larger than `inner` in both dimensions, or FieldOverflow. Scan it for `FRAME_COLOR` (exact RGBA) and return FrameColor at the first one found. Then clear `inner` and copy the bitmap's pixels as they are (RGBA) to the top-left.

**Tests:**
- Unit tests in `write.rs` per spec scenario, on small synthetic images with framed fields and marks (reusing the `detect`/`types` test helpers).
- An integration test `tests/write_example3.rs` loads `testdata/example3.png` in memory, writes F2 (counter 3), F3 (float 33.3 → 177/532), F6 (bool false), F8 (string "HELLO") and F1 (a 10×10 image). After each write it re-runs `detect` and checks that fields' rectangles, links and records are unchanged and only the written value changed. It also checks the error cases from the spec (F2 113, F8 "HELLO WORLD", F5 ref, F2 with a string, field 99), comparing image bytes to confirm nothing changed.

## Risks / Trade-offs

- [Leading/trailing spaces and blank lines are lost by a string write] → They have no pixel representation under the reading rules. The read-back value makes it visible to the caller.
- [A line written with only short glyphs, such as `-`, reads back as `�`] → This is an existing reading limitation (no 5-row anchor). The read-back shows it.
- [Bitmaps with transparency are copied as-is] → They're stored exactly as given. Transparent pixels differ from the background color, so they don't count as "empty" for other types, but `image` fields have no value.
- [`layout` passed in could be stale or from a different image] → Documented: pass the layout of the same image. The write only uses the field's rectangles and its `data` type.
