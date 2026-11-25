# Tasks

## 1. Data model

- [x] 1.1 In `layout.rs`, add `FieldData` (`Image`, `Float { color, filled, total }`, `Counter { color, count }`) with `type_name()` and `percent()`, plus `pub data: FieldData` on `Field`. Re-export `FieldData` from `lib.rs`, and fill `data: FieldData::Image` in the `cli` unit test. Verify unit tests for `type_name()` and `percent()` (0, 196/532 ≈ 36.84, 100) pass and `cargo build --workspace` succeeds.

## 2. Reading types and values

- [x] 2.1 Add `types.rs` with the mark-area lookup, mark color, shape matching against the `FLOAT_MARK`/`COUNTER_MARK` constants and the `image` fallbacks. Call it from `detect` for every field. Verify unit tests pass for: mark position, no room above (top < 3), float mark, counter mark, other pixels being a mix of white and black, no mark, an unrecognized shape and a frame-colored mark.
- [x] 2.2 Implement the float value (mark-color pixels / data-area pixels). Verify unit tests pass for 25 of 100 pixels with black pixels present (25%), an empty area (0%) and a full area (100%).
- [x] 2.3 Implement the counter value (4-connected groups of the mark color inside the data area). Verify unit tests pass for 3 separate blobs (3), two diagonal-only pixels (2), red and blue blobs counting only red (2) and an empty area (0).

## 3. Reference image

- [x] 3.1 Extend `crates/backend/tests/example1.rs` with the types and values from the spec: (101,22)–(137,47) counter 5, (156,22)–(193,50) float 196/532, (130,69)–(167,97) counter 3, and the other two image. Verify `cargo test -p backend --test example1` passes.

## 4. cli

- [x] 4.1 Append `  image`, `  float {:.2}%` or `  counter N` to each field line in `cli`. Update the `cli` integration tests so the reference image's F1 line ends with `counter 5`, F2 with `float 36.84%` and F3 with `image`. Verify `cargo test -p cli` passes, and that `cargo run -q -p cli -- testdata/example1.png` shows the values.

## 5. Workspace checks

- [x] 5.1 Run `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Verify all succeed with no warnings.

## Workflow follow-up

- Archive with `/openspec-archive-change`, then commit everything in one commit (per the project's archive guidance).
