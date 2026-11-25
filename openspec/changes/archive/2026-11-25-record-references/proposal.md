# Proposal

## Why

Records are separate groups of fields, but real data relates records to each other (an order points to a customer, a node to its parent). badba1 needs a way to draw such a relation in Paint: a `ref` field whose `+` mark has a line leading to another record.

## What Changes

- New field type **`ref`**, marked `.#.` / `###` / `.#.` (a "+"). Whatever is drawn in a `ref` field's data area is ignored.
- **Reference line:** pixels of exactly the mark's color, outside every field's frame and data area, connected to the mark.
  - Pixels count as connected when they are at most 5 pixels apart horizontally and at most 5 vertically, which allows gaps of up to 4 pixels. Such gaps appear, for example, where another line is painted across it.
- **Target:** the line touches a field when one of its pixels is within that same distance of the field's frame. That allows the line to stop up to 4 pixels short of the frame.
  - The reference points to the **record** of the touched field, not to the field itself.
  - The referencing field's own record is ignored.
  - If the line touches fields of exactly one other record, that record is the target.
  - If it touches none, or fields of two or more other records, the reference is **null**.
- `cli` shows `ref R<n>` or `ref null` at the end of a `ref` field's line.
- The `+` mark, which previously had no meaning (`image`), now means `ref`.
- Expected result for `testdata/example2.png`:
  - The field at (198,26)–(220,43), F5 with a yellow `+`, references **R2**, the record containing the field at (250,23)–(280,52).
  - The field at (213,74)–(234,93), F10 with a purple `+`, also references **R2**.
  - The purple line has a 1-pixel gap where the yellow line crosses it, and still resolves.

## Capabilities

### New Capabilities
- `references`: How a `ref` field's line is traced, which record it points to, and when the reference is null.

### Modified Capabilities
- `field-types`: Recognizes the `+` shape as the `ref` type. The "Plus-shaped mark" scenario now says the `+` gives `ref`, no longer `image`. A `ref` field's data area is ignored.
- `cli`: The field listing shows `ref R<n>` / `ref null`.

## Impact

- `crates/backend`: `FieldData::Ref { color, record: Option<usize> }`, plus a new `references.rs` that resolves targets after records are computed in `detect`. No new dependencies.
- `crates/cli`: a new `match` arm and tests.
- Tests: synthetic unit tests for every reference rule, and a reference expectation for `testdata/example2.png`, extending `tests/example2.rs`.
