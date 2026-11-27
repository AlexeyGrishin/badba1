# Tasks

## 1. Behavior lock

- [x] 1.1 Add `crates/cli/tests/golden.rs` that runs `cli` on `testdata/example1.png`, `example2.png`, `example3.png` and their 24-bit and 32-bit BMP copies, comparing against golden files in `crates/cli/tests/golden/` captured from the current build; verify `cargo test -p cli` passes before any refactor
- [x] 1.2 Add `crates/server/tests/golden.rs` covering `GET /api/dbs`, records pages, field image bytes, value and image writes (response JSON and resulting file bytes, PNG and BMP), and error responses for the example databases, compared against goldens captured from the current build; verify `cargo test -p server` passes before any refactor

## 2. Storage formats

- [x] 2.1 Create `crates/backend/src/formats/mod.rs` with the `Format` and `Storage` traits, the `FORMATS` registry, `format_for(path)` (extension in any case, PNG fallback) and the single `MAX_PIXELS` header check; add unit tests for extension choice and registry contents; verify `cargo test -p backend formats`
- [x] 2.2 Move PNG reading from `load.rs` into `formats/png.rs` with an in-memory `Storage` whose `write_rect` saves through a temp file + rename and restores pixels on failure (logic from the server's `write_png`/`save`); move its unit tests along and add a save-failure test; verify they pass
- [x] 2.3 Move `bmp.rs` into `formats/bmp.rs` with an on-disk `Storage` (`read_rect`, `stored` forcing opaque alpha without an alpha mask, in-place `write_rect` that restores old bytes on failure, from the server's `write_bmp`); move its unit tests and `tests/bmp_layout.rs` expectations; verify they pass
- [x] 2.4 Make the load error type format-neutral (`Decode`/`Unsupported` carry the format name) with identical message texts; delete `load.rs`, `is_bmp`, `load_image`; verify `cargo test --workspace` and both golden suites pass

## 3. Field types and generic values

- [x] 3.1 Create `crates/backend/src/types/mod.rs` with the `FieldType` trait, `Value`, `Input`, `InputKind`, `Mark`, `mark_area`, `AreaView`, `ResolveCtx`, the `TYPES` registry and `recognize()`; add unit tests that mark shapes are unique and include the top-middle pixel, and for the generic recognition scenarios (no mark, unrecognized shape, frame-colored mark); verify they pass
- [x] 3.2 Add `types/image.rs`, `types/float.rs`, `types/counter.rs` and `types/bool.rs`, each with its mark, `read`, `plan`, `input_kind` and `cli_text`, moving the matching code and unit tests from `types.rs` and `write.rs`; verify each module's tests pass
- [x] 3.3 Add `types/string/` (`read.rs` from `strings.rs`, `write.rs` from `write.rs`'s string planning) with its tests; verify the strings and string-write tests pass unchanged
- [x] 3.4 Move `FieldMap` to `field_map.rs` and add `types/reference.rs` (the `ref` type, not writable, with `resolve` from `references.rs`); move its tests; verify the references tests pass unchanged
- [x] 3.5 Replace `FieldData` with `kind`/`color`/`value` on `Field`; make `detect` call `recognize`, `read` and then `resolve` through the registry; move the shared clear-and-draw code into `draw.rs` and rebuild `write_field` on it with `TypeMismatch` value-type names taken from the registry; delete `types.rs`, `strings.rs`, `references.rs`, `write.rs`; verify `cargo test -p backend` passes
- [x] 3.6 Check type isolation: search `crates/*/src` outside `types/` for per-type names, marks or value handling and confirm only the registry and tests remain; verify by recording the search command and its empty result in the PR description

## 4. Database facade

- [x] 4.1 Add `error.rs` with one `Error` enum (read, decode, unsupported, too large, no record, no field, write, save) keeping today's message texts; verify unit tests for each message
- [x] 4.2 Add `db.rs` with `load_db`, `Database::load`, size/background/layout accessors, `RecordId` (`"x/y"`, `Display`/`FromStr`), `records`, `record`, `field`, `field_image`, and `#[doc(alias)]` for `get_records`/`get_field`; add tests for the library-api read scenarios (example1/example3 records, ids, not-found errors, field image, values of each kind, CLI text); verify they pass
- [x] 4.3 Implement `Database::set_field` per design §5 (not-writable check first, `stored`, plan, draw, `write_rect`, read-back, update in memory); add tests for write-and-reload on PNG and BMP, failed write, and failed save rollback; verify they pass
- [x] 4.4 Port `crates/backend/tests/*.rs` (example1–3, bmp_layout, large_image, write_example3) to `load_db`/`Database` or `detect`; verify `cargo test -p backend` passes, including the large-image tests

## 5. Clients on the facade

- [x] 5.1 Rewrite `crates/cli/src/main.rs` to use `load_db` and each field's `cli_text`, with no per-type `match`; verify `cargo test -p cli` and the CLI golden suite pass
- [x] 5.2 Rewrite `crates/server/src/api.rs` to cache `Database` handles per id (same lazy load, retry and `RwLock` behavior), convert JSON by `InputKind`/`Value` kind, list databases from `FORMATS` extensions, and map `backend::Error` variants to today's statuses and error kinds; remove `Kind`, `Pixels`, `write_png`, `write_bmp`, `find_field`, `record_ids`; verify `cargo test -p server` and the REST golden suite pass
- [x] 5.3 Update the server module docs (`api.rs`, `lib.rs`) to describe it as a client of `backend::Database`; verify `cargo doc -p server` builds without warnings

## 6. Documentation

- [x] 6.1 Add `#![warn(missing_docs)]` to `crates/backend/src/lib.rs` and document every public item; verify `RUSTDOCFLAGS="-D missing_docs" cargo doc -p backend --no-deps` succeeds
- [x] 6.2 Write the crate docs: overview, entry point, "Adding a field type" and "Adding a storage format" sections pointing at `TYPES`/`FORMATS`, and a runnable example that copies `testdata/example3.png` to a temp folder, loads it, reads a field and writes one; verify `cargo test -p backend --doc` passes

## 7. Integration check

- [x] 7.1 Run `cargo test --workspace` and `cargo clippy --workspace --all-targets` at the repository root; verify all tests (including both golden suites) pass with no new warnings
- [x] 7.2 Start the server on `db/` and check in a browser that the web UI lists, pages, edits a counter, float, bool and string, and draws in an image field on a PNG and a BMP database as before; verify by screenshots in the PR description
- [x] 7.3 Run `openspec validate rearchitecture --strict`; verify it reports the change as valid

## Workflow follow-up

- Archive the change after review (`openspec archive rearchitecture`), then delete the emptied `openspec/specs/strings/` and `openspec/specs/references/` folders, whose requirements now live in `openspec/specs/types/string/` and `openspec/specs/types/ref/`.
- Per project guidance, commit code and `openspec/` together in one commit named after the change.
