# Proposal

## Why

badba1 is a database meant to be embedded and extended, but today it has no library API: the in-memory database, record ids, field lookup, write-with-rollback and the PNG/BMP storage strategies all live inside the web server (`crates/server/src/api.rs`), and a project that wants to use badba1 must re-implement them. Knowledge about each data type and each file format is also spread across many files (`types.rs`, `write.rs`, `strings.rs`, `references.rs`, `layout.rs`, the server's JSON code, the CLI's printing code; `load.rs`, `bmp.rs`, the server's `Kind`/`Pixels`), and across several specs, so adding a type or a format means editing all of them.

## What Changes

- Add a documented, single entry point to the `backend` library: a `Database` handle with operations to load a database (`load_db`), list and get records (`records`, `record`), get a field and its value or image (`field`, `field_image`), and write a field (`set_field`). The handle keeps the database in memory, owns the write/save/rollback logic, and uses the file's storage format. Every public item gets rustdoc, and the crate docs show a runnable example.
- Move the in-memory database, record ids (`"x/y"`), field lookup by record and index, and the PNG/BMP save strategies out of the server into the library. The server and CLI become thin clients of the `Database` API.
- **BREAKING (library API only)**: replace the closed `FieldData` enum and `WriteValue` enum with a field-type extension point. Each data type (`image`, `float`, `counter`, `bool`, `ref`, `string`) becomes one module under `crates/backend/src/types/` that holds its mark shape, how its value is read, how a value is written, and how it is shown. A registry lists the types; values are exposed as a small set of generic value kinds, so clients (server JSON, CLI text) need no per-type code.
- **BREAKING (library API only)**: replace the hard-coded PNG/BMP choice (`load_image`, `is_bmp`, the server's `Kind`) with a storage-format extension point. Each format (`png`, `bmp`) becomes one module under `crates/backend/src/formats/` that holds how it is recognized, read, and how field pixels are read and written back. A registry lists the formats.
- Restructure the specs the same way: one spec per data type under `openspec/specs/types/<name>/`, one spec per storage format under `openspec/specs/formats/<name>/`, plus framework specs for the library API, field types, and storage formats. Requirements are moved, not changed.
- No observable behavior changes: CLI output, REST responses, web UI, read/write results, errors and file contents stay identical. The crate keeps the name `backend`.

## Capabilities

### New Capabilities
- `library-api`: the library's single entry point: loading a database, records and record ids, getting fields, values and images, writing fields, in-memory semantics and save rollback, and API documentation.
- `storage-formats`: the storage-format framework: how a file's format is chosen, the maximum image size, and what every format module must provide.
- `types/image`: the `image` data type (default type; bitmap writes).
- `types/float`: the `float` data type (mark, fill-percentage value, writes).
- `types/counter`: the `counter` data type (mark, object-count value, writes).
- `types/bool`: the `bool` data type (mark, value, writes).
- `types/ref`: the `ref` data type (mark, reference lines, targets, null references; not writable). Takes over the `references` requirements.
- `types/string`: the `string` data type (mark, text reading, writes and write errors). Takes over the `strings` requirements.
- `formats/png`: the PNG storage format (reading, in-memory pixels, whole-file save through a temporary file).
- `formats/bmp`: the BMP storage format (supported variants, pixels kept on disk, in-place rectangle writes).

### Modified Capabilities
- `field-types`: becomes the field-type framework: mark location, generic mark recognition against registered types, default `image` type, and the one-module-per-type rule. Per-type marks and values move to `types/*`.
- `field-writes`: keeps generic write rules (scope, clearing, failed writes, wrong field or type); per-type write rules move to `types/*`.
- `strings`: all requirements move to `types/string`.
- `references`: all requirements move to `types/ref`.
- `image-layout`: PNG and BMP reading move to `formats/*`; the maximum image size moves to `storage-formats`. Detection stays.
- `rest-api`: database files are those of any registered format; record ids are the library's; BMP storage details move to `formats/bmp`.

## Impact

- Code: `crates/backend/src` is reorganized (`types/`, `formats/`, `db.rs` facade; `types.rs`, `write.rs`, `strings.rs`, `references.rs`, `load.rs`, `bmp.rs` are split or moved). `crates/server/src/api.rs` loses its storage, lookup and per-type code and uses `backend::Database`. `crates/cli/src/main.rs` uses the API and the types' text form.
- Public Rust API of `backend` changes (new `Database`, `Value`, `FieldType`, `Format`; `FieldData`/`WriteValue` and format-specific functions removed or made internal). The CLI, server and tests in this repository are the only users and are updated.
- Specs: 10 new spec folders; `strings` and `references` are emptied and deleted at archive.
- No new dependencies; no change to files on disk, the REST API, or the web UI.
