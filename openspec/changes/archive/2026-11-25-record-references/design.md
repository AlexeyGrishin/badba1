# Design

## Context

`detect` builds fields and reads each field's `FieldData` from its mark (`types::read_data`), then finds links and records. A reference needs records to resolve, so it can only be finished after `records()` has run. See proposal.md for motivation and `specs/references/spec.md` for the rules.

`testdata/example2.png` (539×348) was analyzed during planning with a throwaway prototype of these rules (distance 5, own record ignored):
- **F5** (198,26)–(220,43): yellow `+` (255,201,14) and a 96-pixel line, which ends next to the bottom frame of F4 (250,23)–(280,52) at (264,53). It touches fields {F4, F5}. Ignoring its own record R1 leaves R2, so the target is **R2**.
- **F10** (213,74)–(234,93): purple `+` (163,73,164) and a 39-pixel line, which ends next to F4's left frame at (249,51). It has a 1-pixel gap at y=61 where the yellow line is painted over it. With plain 8-connectivity the line splits in two; with the 4-pixel gap allowance it is one line. It touches {F4, F10}, so the target is **R2**.

## Goals / Non-Goals

**Goals:**
- Resolve references from the drawing alone, tolerating freehand imperfections (small gaps, stopping short of a frame).
- Keep the per-field reference result simple: `Option<record id>`.

**Non-Goals:**
- References to fields, or to multiple records.
- Validating or "linting" reference lines, such as warning about near misses.
- Using a `ref` field's data area for anything.

## Decisions

**Two-phase type reading.** `read_data` recognizes the `+` shape (`REF_MARK`) and returns `FieldData::Ref { color, record: None }` without looking at the data area. After `records()` runs, `detect` calls `references::resolve(image, &mut fields, &records)`, which fills in `record` for each `Ref` field. This keeps `types.rs` free of record knowledge, and leaves `read_data` pure per field.

**Line tracing (`references.rs`):**
- *Line mask:* pixels exactly equal to the mark color, excluding pixels inside any field's outer rectangle. A boolean grid built once per distinct mark color, so fields sharing a color share the mask.
- *Gap tolerance:* BFS from the mark pixels, where neighbors are all mask pixels within Chebyshev distance ≤ `REF_REACH` (= 5), i.e. a gap of up to 4 pixels. Each step checks an 11×11 window, which is O(line pixels × 121), negligible for hand-drawn lines.
- *Touching:* build a frame-owner grid once (`Option<field id>` for each frame pixel, the same data the link detection uses). For each line pixel, scan the same 11×11 window and collect the frame owners. Map owners to record ids, remove the own record, and the target is `Some(r)` if exactly one record remains, else `None`.
- `REF_REACH` is a named constant in one place.

*Alternative:* dilating the line by 2 pixels and then using 8-connectivity. Rejected because it's harder to state exactly and to match with the spec's "at most 5 pixels apart".

**Distance metric = Chebyshev (square window).** It matches the spec wording ("at most 5 horizontally and at most 5 vertically") and is cheap. Euclidean would allow slightly less on diagonals, which is invisible in practice but harder to explain.

**Data model:** `FieldData::Ref { color: [u8; 4], record: Option<usize> }`, and `type_name()` returns `"ref"`. `cli` prints `ref R{n}` or `ref null`.

**Tests:**
- Unit tests in `references.rs` build images with fields, links (frame-color strokes) and colored reference lines, one per spec scenario: gap 4 vs 5, stopping 4 vs 5 short, any field of the record, two fields of one record, own record ignored, null cases, branching to two records, crossing lines, and pixels inside fields being excluded.
- `types.rs` gets a ref-mark test and a "data area ignored" test, and the plus-shape test changes from `image` to `ref`.
- `tests/example2.rs` adds the two references.
- `cli` gets a unit test for `ref R2` / `ref null`.

## Risks / Trade-offs

- [A reference line passing within 5 pixels of an unrelated field also "touches" it, which could make the reference null] → This follows from the chosen allowance. Draw lines a little away from other frames. On the reference image, neither line comes that close to another field.
- [Two `ref` fields with the same color and lines close together merge into one line, so both reach the union of targets] → Use distinct colors for nearby references. This is documented here, not specified.
- [A field's own `+` touches its own frame, and other fields of its own record may be near] → The own record is always ignored, so this never affects the target.
- [`FieldData` gains a variant, so exhaustive matches outside the crate break] → Only `cli` matches on it, and it's updated here.
