# Proposal

## Why

So far badba1 only finds where fields are. To work as a database it has to read what they hold, and different fields hold different kinds of data. A small type mark drawn next to each field says how to read its contents, and is easy to draw in Paint.

## What Changes

- Every field gets a **data type**, set by a 3×3 pixel mark just outside the field: directly above the frame's top-right corner, its right column level with the frame's right edge and its bottom row touching the frame's top edge. The mark's color is the color of its top-left pixel.
- Three types:
  - **`image`** is the default, used when there's no recognizable mark. The data area is just a bitmap.
  - **`float`**, marked `###` / `##.` / `#..` (looks like "F"). The value is the percentage of data-area pixels that are exactly the mark's color.
  - **`counter`**, marked `###` / `#..` / `###` (looks like "C"). The value is the number of separate objects in the data area drawn in the mark's color. Pixels join into one object only through side-by-side contact; diagonal contact doesn't join them.
- Inside the data area, pixels of any other color count as empty.
- A mark that doesn't match either shape, a mark drawn in the frame color, or no room above the field means the field is `image`.
- `cli` shows each field's type. It shows the value for `float` and `counter` fields; for `image` fields it shows the type and the data-area size it already prints.
- The reference image `testdata/example1.png` gets new expectations:

  | Field | Type | Value |
  |---|---|---|
  | dots box | counter | 5 |
  | green box | float | 36.84% (196 of 532 pixels) |
  | red-dots box | counter | 3 |
  | smiley | image | |
  | frown | image | |

## Capabilities

### New Capabilities
- `field-types`: Data types of fields: how a field's type is read from its mark, and how `float` and `counter` values are computed from its data area.

### Modified Capabilities
- `cli`: The field listing also shows each field's type and, for `float` and `counter` fields, its value.

## Impact

- `crates/backend`: `Field` gains a data-type/value member (a breaking change to the struct, which has only one user so far: `cli`). There's new code to read type marks and compute values. No new dependencies.
- `crates/cli`: the field line format gets the type and value appended. Its tests are updated.
- Tests: new unit tests with synthetic images, and new expectations for `testdata/example1.png`.
