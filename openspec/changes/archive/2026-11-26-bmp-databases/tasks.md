# Tasks

## 1. BMP module (backend)

- [x] 1.1 Add `crates/backend/src/bmp.rs` with `BmpInfo::parse` (the header variants, bit depths and masks from design Decision 1) and `row_offset`. Add `Error::UnsupportedBmp { path, reason }` to `load.rs`. Add unit tests for: bottom-up and top-down headers; 24-bit and 32-bit; RLE and 8-bit palette rejected with the reason; bad signature; and the 20001×20000 header giving `TooLarge`. Verify with `cargo test -p backend bmp`.
- [x] 1.2 Add `save_bmp` (24-bit, bottom-up, 40-byte header) and `load_bmp`. Add a round-trip unit test: an image with odd widths, to exercise row padding, is saved and loaded back identical, apart from alpha becoming 255. Verify with `cargo test -p backend bmp`.
- [x] 1.3 Add `read_rect` and `write_rect` working on an open `File`. Add unit tests on a saved 24-bit image, and on a hand-built 32-bit top-down file:
  - a read rectangle equals the same crop of `load_bmp`;
  - after a write, only that rectangle's bytes change, and the file size stays the same.

  Verify with `cargo test -p backend bmp`.
- [x] 1.4 Add `load_image` and `analyze_image`, choosing the format from the extension in any letter case, and export them together with the `bmp` API from `lib.rs`. Add an integration test in `crates/backend/tests/`: `example1.png`, `example2.png` and `example3.png` are saved as BMP (24-bit bottom-up, 24-bit top-down, 32-bit) into a temporary folder, and `analyze_image` gives the same `Layout` as the PNG. Verify with `cargo test -p backend`.

## 2. CLI

- [x] 2.1 Switch `crates/cli/src/main.rs` to `analyze_image` and update its usage text and doc comment. Add a CLI test: `example1` saved as BMP prints the same output as the PNG, apart from the path. Verify with `cargo test -p cli`.

## 3. Server

- [x] 3.1 Make `db_path` return the path and kind, giving a 409 `AmbiguousDatabase` error when both files exist (add `ApiError::conflict`). Make `list_dbs` collect both extensions, de-duplicated. Add tests for listing BMP databases and for the same id in both formats. Verify with `cargo test -p server`.
- [x] 3.2 Change `Db` to hold `background` and `Pixels::{Memory, Bmp}` (design Decision 3). A BMP is scanned on first use and its image dropped. `records_page` uses `background`, and `get_image` reads from the BMP. Add a test: `example3` saved as BMP gives the same records page and field image as the PNG. Verify with `cargo test -p server`.
- [x] 3.3 Implement the BMP write path with the field window (design Decision 4), alpha flattening for uploads (Decision 5), `sync_data`, and restoring the old bytes when the write fails. Add tests:
  - a counter write gives the same file size, only data-area bytes change, and `analyze_image` reads the new value;
  - an image upload reads back with opaque alpha;
  - an `UnknownCharacter` or overflow error leaves the file unchanged;
  - with the file made read-only, a write returns 500 and memory is unchanged.

  Verify with `cargo test -p server`.

## 4. Tools and large-database check

- [x] 4.1 Make `gen_big` write a BMP (via `backend::save_bmp`) when the output path ends in `.bmp`. Verify that `cargo run --release -p backend --example gen_big -- db/big.bmp` writes a 10000×40000 file, and that `cli db/big.bmp` prints 72000 fields and 9000 records.
- [x] 4.2 Start a release `server --db db --port 8099` and request `/api/dbs/big/records` (from `big.bmp`; move `big.png` aside temporarily so the id is not ambiguous, then restore it). Record:
  - the first-request time;
  - the time of the next records page and a field image, each expected under 100 ms;
  - the process working set after the first request, expected under 500 MB;
  - the time of one counter write.

  Then stop only that server process.

## 5. Final check

- [x] 5.1 Run `cargo test --workspace`, `cargo clippy --workspace --all-targets` and `cargo fmt --all -- --check`. Verify that everything passes with no warnings.

## Workflow follow-up

- Archive the change after review, then commit code and `openspec/` together in one commit named after the change.
