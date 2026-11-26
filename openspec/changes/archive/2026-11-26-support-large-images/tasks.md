# Tasks

## 1. Baseline

- [x] 1.1 Save the current `cli` output for `testdata/example1.png`, `example2.png` and `example3.png` to a scratch folder, to compare against later. Verify that three non-empty files exist.

## 2. Size-checked loader (backend)

- [x] 2.1 Add `pub const MAX_PIXELS: u64 = 400_000_000` and `Error::TooLarge { path, width, height }` in `crates/backend/src/load.rs`, with the Display text from design Decision 1. Export both from `lib.rs`. Verify with `cargo build -p backend`.
- [x] 2.2 Add `pub fn load_png(path) -> Result<RgbaImage, Error>`. It reads the bytes and checks the dimensions from the IHDR header via `into_dimensions()`. Over the limit it returns `TooLarge`. Otherwise it decodes with `max_alloc = MAX_PIXELS * 8` and `into_rgba8()`. Make `analyze_png` call it. Verify that the existing `load.rs` tests still pass.
- [x] 2.3 Add unit tests that use a hand-built header-only PNG (signature plus an IHDR with a correct CRC). The 20001×20000 case gives `TooLarge`, with a message that contains `20001x20000` and `400000000` and names the path. The 100000×100000 case gives `TooLarge` and returns immediately. Verify with `cargo test -p backend load`.

## 3. Bounded-memory detection (backend)

- [x] 3.1 Change `detect::Grid::regions` to pass `visit` a bounding rect and pixel count instead of a pixel list, and update `find_fields` and `filled_rect` to match. Verify that `cargo test -p backend` passes unchanged.
- [x] 3.2 Add a shared `FieldMap` (`owner: Vec<u32>` with 0 for none, `inside: Vec<bool>`), built once in `detect()` after fields are numbered. Use it in `find_links` instead of `Vec<Option<usize>>`, and pass it to `references::resolve` in place of `references::Grid`'s own `inside` and `frame_owner`. Verify that `cargo test -p backend` passes, including `tests/example*.rs`.
- [x] 3.3 Replace the per-ref full-image `seen` buffer in `references::Grid::trace` with a `HashSet<(u32, u32)>`. Verify that the reference tests in `references.rs` and `tests/example3.rs` pass.
- [x] 3.4 Drop `Grid.mask` and every `seen` buffer as soon as its stage ends (design Decision 6). Verify with `cargo test -p backend` and `cargo clippy -p backend` with no new warnings.
- [x] 3.5 Compare the `cli` output on the three testdata images with the baseline from 1.1. Verify that they match byte for byte.

## 4. CLI and server

- [x] 4.1 Add a CLI integration test in `crates/cli/tests/cli.rs`. Running `cli` on a header-only 20001×20000 PNG exits with status 1, and stderr names the file and contains "too large". Verify with `cargo test -p cli`.
- [x] 4.2 Replace `crates/server/src/api.rs::load` with `backend::load_png`. Map `Error::TooLarge` to a 500 response with `"error": "ImageTooLarge"`, and other errors to `Internal`. Verify with `cargo build -p server`.
- [x] 4.3 Add a server test in `crates/server/tests/api.rs`. With a header-only `huge.png` in the db folder, `GET /api/dbs/huge/records` returns 500 with `"error": "ImageTooLarge"`. Verify with `cargo test -p server`.

## 5. Large-image regression

- [x] 5.1 Commit `crates/backend/examples/gen_big.rs` and make sure it still builds after the API changes. Verify with `cargo build --release -p backend --example gen_big`.
- [x] 5.2 Add the `#[ignore]` test `crates/backend/tests/large_image.rs` from design Decision 7. It uses `db/big.png` or `BADBA1_BIG`, skips with a message if the file is missing, and asserts 10000×40000, 72000 fields and 9000 records. Document how to run it in the file's doc comment. Verify with `cargo test --release -p backend -- --ignored large_image` after regenerating `db/big.png` with `cargo run --release -p backend --example gen_big`.
- [x] 5.3 Run `cli db/big.png` from a release build and record the wall time and peak working set (for example, PowerShell `Get-Process` peak `PeakWorkingSet64`). Verify that it exits 0, prints `db/big.png: 10000x40000`, takes under 60 s and peaks under about 5 GB, and note the numbers in the commit message.
- [x] 5.4 Start `server --db db` and request `/api/dbs/big/records`. Verify that it returns 200 with 20 records.

## 6. Final check

- [x] 6.1 Run `cargo test --workspace` and `cargo clippy --workspace`. Verify that everything passes with no new warnings.

## Workflow follow-up

- Archive the change after review. Per project guidance, commit code and `openspec/` together in one commit named after the change.
