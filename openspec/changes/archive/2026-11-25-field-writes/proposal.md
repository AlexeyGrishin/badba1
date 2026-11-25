# Proposal

## Why

badba1 can only read its image so far. To act as a database it must also change values: draw a new count, percentage, flag, text or picture into a field, so the image stays editable by both Paint and the API. Field geometry (frames, sizes, links) stays as it is. Only data areas change.

## What Changes

- New backend API for writing one field's value into an image in memory. It's library only for now, with no `cli` command. The write replaces the field's data area and returns the field's value **as read back** after writing.
- Every write first clears the data area to the background color (the image's top-left pixel), then draws in the field's mark color. A failed write leaves the image unchanged.
- Per type:
  - **counter** `N`: draws N separate squares on a grid in reading order, with the largest square size that fits and 1-pixel gaps. `N = 0` clears the area. If even 1×1 squares can't fit N, the write fails with **FieldOverflow**.
  - **float** `N`: N is a percentage from 0 to 100. The write fills round(N × area / 100) pixels, column by column from the left. The returned value is what was actually stored, which may differ from N. Values outside 0–100 are an error.
  - **string** text: letters (main form from the glyph font) from the top-left, 1 empty column between letters. A space adds 3 empty columns, and `\n` moves to x = 0, y + 6. Text doesn't wrap. A letter that doesn't fit gives **FieldOverflow**, and a character not in the font gives **UnknownCharacter**.
  - **bool**: `true` fills the whole data area with the mark color, and `false` clears it.
  - **image** bitmap: must be no larger than the data area (otherwise **FieldOverflow**) and must not contain the frame color `#BADBA1` (otherwise rejected). It replaces the area's contents, placed at the top-left, with the rest cleared.
  - **ref**: not writable, so the write fails.
- Writing a value of the wrong type for the field (for example text into a counter) is an error.

## Capabilities

### New Capabilities
- `field-writes`: Writing values into fields' data areas: the shared rules (clearing, colors, read-back, no change on error) and per-type drawing and errors.

### Modified Capabilities
<!-- None. Reading rules are unchanged; written fields are read with the existing specs. -->

## Impact

- `crates/backend`: new `write.rs` with `write_field(image, layout, field_id, value) -> Result<FieldData, WriteError>`, `WriteValue` and `WriteError`. It reuses `types::read_data` for the read-back and the built-in font for strings. No new dependencies.
- No change to `cli` or to existing reading behavior.
- Tests: synthetic unit tests per type and error, plus round-trip tests on an in-memory copy of `testdata/example3.png`. Those tests write each writable field, re-detect, and check the layout is unchanged apart from the written values.
