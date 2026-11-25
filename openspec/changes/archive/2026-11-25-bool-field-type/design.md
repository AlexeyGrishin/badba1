# Design

## Context

`types.rs` reads the mark color from the top-left pixel of the mark area, builds a 3×3 shape of pixels equal to it, and compares that against `FLOAT_MARK` and `COUNTER_MARK`. The `b` mark the user supplied (a 3×3 PNG, pink `#FFAEC9`) is `.#.` / `.##` / `.##`. Its top-left pixel is empty, so the current rule can't recognize it. See proposal.md for motivation and `specs/field-types/spec.md` for the rules.

The user's updated `db/example1.png` was checked during planning. It has 10 fields, 6 links and 3 records:
- (310,29)–(327,49): pink `b` mark, 88 pink pixels inside → `bool true`
- (288,31)–(305,49): pink `b` mark, no pink inside → `bool false`
- (198,26)–(220,43) and (213,74)–(234,93): `+` marks in yellow and purple. Not a type, so they stay `image`.
- The other fields keep their types: the counter at (101,22) is now 4, and the float at (156,22) is now 43.42%, because the user redrew them.

## Goals / Non-Goals

**Goals:**
- Add `bool` with the least change to the mark rules, keeping every existing `float`/`counter` mark readable exactly as before.

**Non-Goals:**
- Defining the `+` mark, or the colored lines in the new image that touch marks. They're left for a later change.
- Replacing `testdata/example1.png`. Its exact expectations (counter 5, float 196/532, records) stay as they are.

## Decisions

**Mark color from the top-middle pixel.** The top-middle pixel is set in all of `FLOAT_MARK`, `COUNTER_MARK` and `BOOL_MARK`. For `float` and `counter`, both the top-left and top-middle pixels belong to the shape, so they always have the mark color. Reading either one gives the same color and therefore the same shape: the change is backward compatible for every image. *Alternative:* "the one non-background color in the area". Rejected because it needs a definition of background, and would misread marks drawn over colored backgrounds.

**Data model:** `FieldData::Bool { color: [u8; 4], value: bool }`, and `type_name()` returns `"bool"`. `percent()` stays float-only.

**Value computation:** an `any()` over the data-area pixels, stopping at the first match.

**`cli`:** a new `match` arm, `FieldData::Bool { value, .. } => format!("bool {value}")`, which prints `bool true` or `bool false`.

**Second reference image:** copy the current `db/example1.png` byte-for-byte to `testdata/example2.png`. A new backend integration test, `tests/example2.rs`, asserts only what the spec pins: the size and the two bool fields. Other details of that image (the `+` marks, colored lines) may get meaning in later changes, so pinning them now would create churn.

**Tests:**
- Unit tests in `types.rs` for the `bool` mark, a black top-left pixel with a pink `b` (top-middle rule), the plus-shaped mark → image, and the four bool value scenarios.
- Existing float/counter tests stay unchanged and confirm backward compatibility.
- A `cli` unit test renders a `bool true` and a `bool false` field.

## Risks / Trade-offs

- [The top-middle rule makes some previously "unrecognized" areas read differently] → Only areas whose top-middle pixel differs from their top-left pixel are affected. Those could never be `float` or `counter` before, and they now only change when the new pixels form exactly the `b` shape.
- [A stray pixel of the mark color inside a `bool` field flips it to true] → That's what the spec asks for: "at least one pixel". It's the intended, Paint-friendly semantics.
- [`example2.png` diverges from `db/example1.png` when the user edits it again] → That's intended: `db/` is for manual testing, and `testdata/` is only updated deliberately.
