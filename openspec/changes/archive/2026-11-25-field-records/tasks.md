# Tasks

## 1. Records in the backend

- [x] 1.1 In `layout.rs`, add `Record { id, fields }` and `Layout.records`. Re-export `Record` from `lib.rs`, and update the `cli` unit test that builds a `Layout`. Verify `cargo build --workspace` succeeds.
- [x] 1.2 Add `records.rs` with `pub fn records(fields, links) -> Vec<Record>`: an adjacency list from links (every pair of a link's fields), components started by the smallest unvisited id, an iterative depth-first walk with ascending neighbors, and numbering in start order. Call it from `detect`. Verify unit tests pass for: a chain, separate groups, a lone field, a 3-field link, position order equal to id order, the first field being the topmost, deeper before later siblings (A, B, D, C), equal-y siblings (A, C, B), a loop of three (A, B, C), two-record numbering, and a 10,000-field chain without stack overflow.

## 2. Reference image

- [x] 2.1 Extend `crates/backend/tests/example1.rs` so record 1 is the fields at (101,22), (156,22), (35,23) in order, and record 2 is (130,69), (72,70). Verify `cargo test -p backend --test example1` passes.

## 3. cli

- [x] 3.1 Add `, N records` to the summary line and a `Records:` section (`  R<id>  F<a>, F<b>, …`) after the links. Update the `cli` integration tests (summary ending `5 fields, 3 links, 2 records`, output ending with `Records:\n  R1  F1, F2, F3\n  R2  F4, F5\n`), and make the `render` unit test check `R1  F1` for a lone field. Verify `cargo test -p cli` passes, and that `cargo run -q -p cli -- testdata/example1.png` shows the records.

## 4. Workspace checks

- [x] 4.1 Run `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Verify all succeed with no warnings.

## Workflow follow-up

- Archive with `/openspec-archive-change`, then commit everything in one commit (per the project's archive guidance).
