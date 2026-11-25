# Proposal

## Why

Many record values are simply yes/no. A `bool` field type lets a user mark something as true by drawing anything in the mark's color inside the field, and as false by leaving it out. It's the easiest possible value to edit in Paint.

## What Changes

- New field type **`bool`**, marked `.#.` / `.##` / `.##` (looks like "b"). The value is **true** when at least one data-area pixel is exactly the mark color, and **false** when there is none. Pixels of other colors don't count.
- **The mark color is now taken from the top-middle pixel of the mark area instead of the top-left.** The `b` mark leaves its top-left pixel empty, so with the old rule it would read as a white mark and fall back to `image`. The top-middle pixel is set in all three shapes, so every existing `float` and `counter` mark reads exactly as before.
- `cli` shows `bool true` or `bool false` at the end of a `bool` field's line.
- A second reference image, `testdata/example2.png`, is a copy of the user's updated `db/example1.png`. Its bool fields are pinned: the field at (310,29)–(327,49) is `bool true` (88 pink pixels inside), and the field at (288,31)–(305,49) is `bool false`. The existing `testdata/example1.png` and its expectations stay unchanged.
- Not included: the `+` mark (`.#.` / `###` / `.#.`) that the updated image uses on two fields. It isn't a defined type yet, so those fields remain `image`, as the existing rules require.

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `field-types`: Mark color comes from the top-middle pixel. Adds the `bool` shape and the bool value. The image fallback lists the `bool` shape among the recognized ones. Adds the second reference image.
- `cli`: The field listing shows `bool true` / `bool false`.

## Impact

- `crates/backend`: `FieldData::Bool { color, value }`. `types.rs` gains `BOOL_MARK`, the top-middle mark color and the bool value. `Field`'s users must handle the new variant (only `cli` does).
- `crates/cli`: one more `match` arm in the field formatting, plus tests.
- `testdata/example2.png` is added (copied from `db/example1.png` as it is now), along with a backend test for it.
