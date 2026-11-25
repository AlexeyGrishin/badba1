# Tasks

## 1. API skeleton

- [x] 1.1 Add `write.rs` with `WriteValue`, `WriteError` (with `Display` and `std::error::Error`) and `write_field(image, layout, field, value) -> Result<FieldData, WriteError>`, implementing the field lookup, `ref` → NotWritable, type mismatch, the clear-then-draw step with background from (0,0), and read-back via `types::read_data`. Export them from `lib.rs`. Verify unit tests pass for an unknown field (99), a ref field, a type mismatch (text into a counter), and that pixels outside the data area are untouched.

## 2. Per-type writes

- [x] 2.1 Counter: largest square size `s` whose grid with 1-pixel gaps holds N, squares in reading order, N = 0 clears, FieldOverflow when `s = 1` doesn't fit. Verify unit tests pass: 3 in 27×16 (3 squares in the first row, reads 3), 0 (cleared, reads 0), 112 in 27×16 (1×1 squares, reads 112), 113 → FieldOverflow with the image unchanged, and old dots replaced (5 → 2).
- [x] 2.2 Float: range check `0..=100` (finite), `round(N × area / 100)` pixels in column-major order. Verify unit tests pass: 33.3 on 532 pixels → 177 filled, returns 177/532; 50 → 266; −1 and 100.5 → OutOfRange with the image unchanged.
- [x] 2.3 Bool: true fills, false clears. Verify a unit test passes for true → false → true read-backs.
- [x] 2.4 String: add `Font::main_form(ch)`, then the cursor layout (1-column letter gap, 3-column space, `\n` → x = 0, y + 6), UnknownCharacter, and FieldOverflow on the right or bottom edge. Verify unit tests pass: `HELLO` at columns 0/4/8/12/16 reads back as `HELLO`, `A B` with 4 empty columns reads `A B`, `AB\nCD` in an 11-row area reads `AB\nCD`, `Hi` → UnknownCharacter('i'), `HELLO WORLD` in 40×5 → FieldOverflow, `AB\nCD` in 40×5 → FieldOverflow, and every error leaves the image unchanged.
- [x] 2.5 Image: size check (FieldOverflow), frame-color scan (FrameColor with position), then clear and copy to the top-left. Verify unit tests pass: 10×10 into 33×20 (top-left equals the bitmap, the rest is background), 33×20 (the whole area equals the bitmap), 34×20 → FieldOverflow, and a bitmap with a `#BADBA1` pixel → FrameColor at its position, with the image unchanged.

## 3. Round trip on the reference image

- [x] 3.1 Add `crates/backend/tests/write_example3.rs`: load `testdata/example3.png` in memory, write F2 = 3, F3 = 33.3, F6 = false, F8 = `HELLO` and F1 = a 10×10 bitmap, check each returned value, and re-detect after each write, checking rectangles, links and records are unchanged and only the written field's value changed. Also check the error cases (F2 = 113, F8 = `HELLO WORLD`, F5 ref, F2 with text, field 99) leave the image bytes identical. Verify `cargo test -p backend --test write_example3` passes.

## 4. Workspace checks

- [x] 4.1 Run `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Verify all succeed with no warnings.

## Workflow follow-up

- Archive with `/openspec-archive-change`, then commit everything in one commit (per the project's archive guidance).
