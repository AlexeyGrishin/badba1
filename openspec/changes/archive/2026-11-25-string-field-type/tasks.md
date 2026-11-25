# Tasks

## 1. Font

- [x] 1.1 Copy this change's `font.png` to `crates/backend/font/font.png` and create `crates/backend/font/chars.txt` with `ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890_+-*`. Verify `cmp` shows the PNG identical to the change's copy.
- [x] 1.2 Add `font.rs`: `Font::parse(png, chars) -> Result<Font, FontError>` (grid 4/6 pitch, top-left background, row 0 main forms, lower rows alternates, empty cells skipped, count-mismatch and shared-shape errors), `Font::lookup(mask) -> Option<char>`, and `builtin()` via `include_bytes!`/`include_str!` and `OnceLock`. Verify unit tests pass: the built-in font has 40 characters and 56 forms, both E forms map to `E`, and synthetic fonts produce a count mismatch and a shared shape error naming both characters.

## 2. String type and text reading

- [x] 2.1 Add `FieldData::String { color, text }` (`type_name()` returns `"string"`), dropping `Copy` from `FieldData` and fixing its uses. Add `STRING_MARK` (`.#.` / `#.#` / `#.#`, exported) in `types.rs`, and the `cli` arm `string {text:?}`. Verify `cargo build --workspace` succeeds, all existing tests still pass, and the string-mark unit test passes.
- [x] 2.2 Add `strings.rs` with `read_text(image, inner, font)`: background from (0,0), 8-connected ink components, line tops from 5-row-tall components, component-to-line assignment, 3-column cells per line, floor(G/3) spaces, exact font lookup or `�`, and lines ordered and joined with `\n`. Use it for `String` fields. Verify unit tests pass for: letters in several colors, a non-white background, 1-column gaps, touching letters, two lines, `A-B` with a short `-`, `TOP\nBOTTOM`, lines beside each other (`LEFT\nRIGHT`), an empty field, both E forms, an unknown glyph, one pixel off, gaps of 1/2 (no space), 3/4/5 (one), 6 (two), and leading/trailing gaps.

## 3. Reference image

- [x] 3.1 Copy the current `db/example1.png` byte-for-byte to `testdata/example3.png`, and add `crates/backend/tests/example3.rs` asserting that (297,54)–(338,60) is `String` with text `OPENSPEC`. Verify `cargo test -p backend --test example3` passes, and `example1`/`example2` still pass unchanged.

## 4. cli

- [x] 4.1 Add a `cli` unit test for `string "OPENSPEC"` and `string "AB\nCD"`. Verify `cargo test -p cli` passes, and that `cargo run -q -p cli -- testdata/example3.png` shows `string "OPENSPEC"` for F8.

## 5. Workspace checks

- [x] 5.1 Run `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Verify all succeed with no warnings.

## Workflow follow-up

- Archive with `/openspec-archive-change`, then commit everything in one commit (per the project's archive guidance). The change's own `font.png` is archived with it, and the built-in copy lives in `crates/backend/font/`.
