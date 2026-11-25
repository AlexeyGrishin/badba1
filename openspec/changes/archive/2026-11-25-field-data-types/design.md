# Design

## Context

`backend::detect` currently returns fields with geometry only (`id`, `outer`, `inner`, `borders`). Detection works on a frame-color mask, but the original `RgbaImage` is still available inside `detect`. `cli` prints one line per field. See proposal.md for motivation and `specs/field-types/spec.md` for the rules.

The type marks in `testdata/example1.png` were checked during planning:
- Every mark sits exactly in the spec's mark area: the right column is level with the frame's right edge, and the bottom row is the row just above the frame.
- The dots box has a green C, the green box a green F, the red-dots box a red C. The smiley and frown have white (empty) mark areas.

A throwaway prototype of the value rules gave: dots counter 5, float 196/532 = 36.842…%, red counter 3. For these images, 4- and 8-connectivity give the same counts.

## Goals / Non-Goals

**Goals:**
- Read the type from the mark and compute values in the same pass as detection, so `Layout` stays the single result of analyzing an image.
- Keep values exact in the library (counts and pixel ratios) and leave rounding to whoever displays them.

**Non-Goals:**
- Writing values back into images, or any API other than `cli`.
- Additional types, or marks in other positions or sizes.
- Exposing the bitmap of `image` fields. Callers can crop `inner` from the image themselves.

## Decisions

**Data model: `Field` gains `data: FieldData`.**
```rust
pub enum FieldData {
    Image,
    Float { color: [u8; 4], filled: u64, total: u64 },
    Counter { color: [u8; 4], count: u64 },
}
impl FieldData {
    pub fn type_name(&self) -> &'static str;   // "image" | "float" | "counter"
    pub fn percent(&self) -> Option<f64>;      // Float only: 100 * filled / total
}
```
Storing `filled`/`total` instead of an `f64` keeps `Field: Eq`, which tests rely on, and keeps the value exact. `percent()` does the division. The color is a plain RGBA array, so callers don't need `image` types. *Alternative:* separate `data_type` and `value: Option<f64>` fields. Rejected because it allows impossible combinations, such as an image with a value.

**New module `types.rs`:** `fn read_data(image: &RgbaImage, field: &Field) -> FieldData`. `detect` calls it for each field after the fields are numbered.
- Mark area: `x in outer.right-2..=outer.right`, `y in outer.top-3..=outer.top-1`. If `outer.top < 3`, the type is `Image`. `outer.right >= 2` always holds, because a field is at least 3 pixels wide.
- Mark color = the RGBA of the top-left mark pixel. If it is `FRAME_COLOR`, the type is `Image`.
- Shape = a 9-bit pattern of which mark pixels equal the mark color exactly. It is compared against two constant patterns, `FLOAT_MARK` (`###/##./#..`) and `COUNTER_MARK` (`###/#../###`). Anything else is `Image`. A one-color area becomes `Image` automatically, because its shape is all 9 pixels.
- Float: count the `inner` pixels equal to the mark color. `total = inner.width() * inner.height()`.
- Counter: count 4-connected groups of mark-color pixels within `inner`, using a flood fill with a visited buffer the size of `inner`.

Exact RGBA equality is used throughout, matching how the frame color is handled.

**`cli` format:** the field line gets two spaces plus the type and value appended:
```
  F1  outer (101,22)-(137,47) 37x26  border t5 r5 b5 l5  data (106,27)-(132,42) 27x16  counter 5
  F2  outer (156,22)-(193,50) 38x29  border t5 r5 b5 l5  data (161,27)-(188,45) 28x19  float 36.84%
  F3  outer (35,23)-(77,52) 43x30  border t5 r5 b5 l5  data (40,28)-(72,47) 33x20  image
```
Floats use `{:.2}%`. The data size the user asked to see for `image` fields is already on the line (`33x20`).

**Tests:**
- Unit tests in `types.rs` draw marks with a small helper (`mark(img, outer, pattern, color)`) on top of the existing `frame` helper from `detect.rs` tests. That helper becomes `pub(crate)`.
- `tests/example1.rs` adds the type and value expectations.
- `cli` tests add the three ending checks.

## Risks / Trade-offs

- [A link stroke or another field that crosses the mark area can accidentally form a valid mark shape] → Unlikely, since it would need exactly the F or C pattern in one color. Accepted.
- [An anti-aliased or slightly-off mark pixel turns the mark into an unknown shape, so the field silently becomes `image`] → That's the behavior you chose. A future "lint" command could flag mark areas that are almost a known shape.
- [A `float` data area that contains text or drawings in other colors] → Those pixels count as empty, as the spec requires. Only the mark color counts.
- [`Field` gains a field, which breaks code that builds `Field` directly] → Only `cli`'s unit test does, and it's updated in this change.
