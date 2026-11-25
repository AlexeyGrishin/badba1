# Tasks

## 1. Ref type

- [x] 1.1 Add `FieldData::Ref { color, record: Option<usize> }` (`type_name()` returns `"ref"`) in `layout.rs`, and the `cli` arm (`ref R<n>` / `ref null`). Verify `cargo build --workspace` succeeds and `type_names` covers `ref`.
- [x] 1.2 In `types.rs`, add `REF_MARK` (`.#.` / `###` / `.#.`, exported) returning `Ref { color, record: None }` without reading the data area. Change the plus-shape test so it expects `ref`. Verify unit tests pass for the ref mark, the plus shape now giving `ref`, and a ref field with yellow and black pixels inside still being `ref`.

## 2. Reference resolution

- [x] 2.1 Add `references.rs` with `REF_REACH = 5` and `resolve(image, &mut fields, &records)`: a line mask (exact mark color, outside all outer rectangles), BFS from the mark pixels within Chebyshev distance 5, frame owners within distance 5, mapping to records, removing the own record, and exactly one remaining → `Some`, else `None`. Call it from `detect` after `records()`. Verify unit tests pass for: a line in the mark color, a line in another color (null), pixels inside fields excluded, a gap of 4 (connected) vs 5 (not), a line crossed by another color, ending at the frame, stopping 4 short (touch) vs 5 short (no touch), touching a later field of the record, touching two fields of one record, own record ignored, reaching nothing, reaching only its own record, and branching to two records (null).

## 3. Reference image

- [x] 3.1 Extend `crates/backend/tests/example2.rs`: (198,26)–(220,43) and (213,74)–(234,93) are `Ref` with the target being the record that contains (250,23)–(280,52). Verify `cargo test -p backend --test example2` passes and `example1` is unchanged.

## 4. cli

- [x] 4.1 Add a `cli` unit test for `ref R2` and `ref null`. Verify `cargo test -p cli` passes, and that `cargo run -q -p cli -- testdata/example2.png` shows `ref R2` for F5 and F10.

## 5. Workspace checks

- [x] 5.1 Run `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Verify all succeed with no warnings.

## Workflow follow-up

- Archive with `/openspec-archive-change`, then commit everything in one commit (per the project's archive guidance).
