# Tasks

## 1. Dependencies and layout types

- [x] 1.1 Add `image = { version = "0.25", default-features = false, features = ["png"] }` and `backend = { path = "crates/backend" }` to root `[workspace.dependencies]`. In `crates/backend/Cargo.toml`, add `image = { workspace = true }`. Verify `cargo build -p backend` downloads `image` and succeeds.
- [x] 1.2 Replace the placeholder in `crates/backend/src/lib.rs` with the modules `layout`, `detect` and `load`, and define `Rect` (inclusive coords, `width()`/`height()`), `Borders`, `Field`, `Link` and `Layout` in `layout.rs`, with doc comments. Verify `cargo test -p backend` passes the unit tests for `Rect::width`/`height`.

## 2. Field detection

- [x] 2.1 In `detect.rs`, implement the frame-color mask (exact RGBA 186, 219, 161, 255) and add a test helper that draws rectangles, lines and circles into an `RgbaImage`. Verify unit tests for the four "Frame color matching" scenarios pass.
- [x] 2.2 Implement interior search (4-connected flood fill of non-mask pixels, keeping filled rectangles that don't touch the edge) and frame growth with per-side thickness. Verify unit tests pass for: equal borders, unequal borders (1/4/7/2), a data area with a drawing, a 1px frame around a 1×1 interior, a closed circle (no field), and a frame-color dot in the interior (no field).
- [x] 2.3 Number fields in reading order (outer top, then left). Verify the "Numbering follows reading order" unit test passes.

## 3. Link detection

- [x] 3.1 Implement strokes (8-connected frame-color pixels outside all frames) and links (strokes touching the frames of ≥2 fields; sorted, unique field lists; links numbered by field list). Expose `pub fn detect(&RgbaImage) -> Layout`. Verify unit tests pass for: a straight 1px link, a thick curved link, a branching link touching 3 fields, a stroke touching only one field (no link), and a line with a 1px gap (no link).

## 4. PNG loading and reference image

- [x] 4.1 In `load.rs`, implement `Error` (`Io`/`Decode`, `Display` naming the path, `std::error::Error` with `source`) and `pub fn analyze_png(&Path) -> Result<Layout, Error>`. Verify unit tests pass for a missing file (Io error naming the path) and a non-PNG file (Decode error).
- [x] 4.2 Add `crates/backend/tests/example1.rs`, which analyzes `testdata/example1.png` (a copy of the user's `db/example1.png`; tests never read `db/`) and asserts the size 539×348, the 5 fields with exact outer rectangles and 5px borders, and the 3 links from the spec. Verify `cargo test -p backend --test example1` passes.

## 5. cli crate

- [x] 5.1 Create `crates/cli` (package and binary `cli`, workspace-inherited metadata, depends on `backend`). Implement argument handling (usage to stderr and exit 2 unless exactly one argument), file errors (message to stderr and exit 1), and the summary/field/link output format from design.md. Verify `cargo run -p cli -- testdata/example1.png` prints 539x348, 5 fields and 3 links.
- [x] 5.2 Add `crates/cli/tests/cli.rs`, running `CARGO_BIN_EXE_cli` for: the reference image (summary and the F1 line `(101,22)-(137,47) 37x26`, borders 5 and data `(106,27)-(132,42) 27x16`, plus links `F1 - F2`, `F1 - F3`, `F4 - F5`), no arguments (exit 2, usage on stderr) and a missing file (exit 1, path on stderr). Verify `cargo test -p cli` passes.

## 6. Workspace checks

- [x] 6.1 Run `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Verify all succeed with no warnings.

## Workflow follow-up

- `testdata/example1.png` is untracked. Include it in the commit, since the tests depend on it. `db/` is ignored by git.
- Archive with `/openspec-archive-change`, then commit everything in one commit (per the project's archive guidance).
