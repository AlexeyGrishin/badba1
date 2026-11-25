# Tasks

## 1. Backend

- [x] 1.1 Add `FieldData::Bool { color, value }` (with `type_name()` returning `"bool"`) in `layout.rs`, and handle it in `cli`'s field formatting (`bool true` / `bool false`). Verify `cargo build --workspace` succeeds and the `type_names` unit test covers `bool`.
- [x] 1.2 In `types.rs`, take the mark color from the top-middle pixel, and add `BOOL_MARK` (`.#.` / `.##` / `.##`, exported from `lib.rs`) and the bool value (any data-area pixel equal to the mark color). Verify all existing float/counter tests still pass, and new unit tests pass for: the bool mark, a black top-left pixel with a pink `b`, the plus-shaped mark → image, one pink pixel → true, a filled area → true, other colors only → false, and an empty area → false.

## 2. Reference images

- [x] 2.1 Copy `db/example1.png` byte-for-byte to `testdata/example2.png`, and add `crates/backend/tests/example2.rs` asserting 539×348, (310,29)–(327,49) `bool true` and (288,31)–(305,49) `bool false`. Verify `cargo test -p backend --test example2` passes and `cargo test -p backend --test example1` still passes unchanged.

## 3. cli

- [x] 3.1 Add a `cli` unit test that renders a `bool true` and a `bool false` field, checking that their lines end with `bool true` and `bool false`. Verify `cargo test -p cli` passes, and that `cargo run -q -p cli -- testdata/example2.png` shows `bool true` for F6 and `bool false` for F7.

## 4. Workspace checks

- [x] 4.1 Run `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Verify all succeed with no warnings.

## Workflow follow-up

- Archive with `/openspec-archive-change`, then commit everything in one commit (per the project's archive guidance).
