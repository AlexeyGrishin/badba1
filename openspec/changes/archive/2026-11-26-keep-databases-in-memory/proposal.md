# Proposal

## Why

The server reads, decodes and analyzes a database's PNG file on every request; writes do this too, then re-encode the file. On small databases this goes unnoticed. On a maximum-size one such as `db/big.png` (10000×40000), every records page or image fetch takes about 8 seconds and briefly needs about 3.6 GB, and concurrent requests each hold their own copy. Databases should be read into memory once and served from there.

## What Changes

- The server keeps each database it has opened in memory: the decoded image and its detected layout, which holds the fields, links, records and record ids. A database is loaded the first time a request names it and stays in memory for the rest of the server's run.
- Reads (records pages, field image data) are served from memory without touching the file.
- Writes change the in-memory image and the field's data, then save the file as today, so the file and memory stay the same. A write does not re-run detection, because a write only changes pixels inside one field's data area. If saving fails, the in-memory change is rolled back.
- Reads of a database can run at the same time. A write to a database waits for its reads to finish, and blocks reads of that database while it runs. Different databases do not block each other.
- **BREAKING (behavior)**: changes made to a database file on disk by another program, such as an image editor, are no longer picked up while the server runs. The server must be restarted to see them. Hot reload is explicitly out of scope for this change.
- A failed load, such as an image that is too large or not a valid PNG, is not cached. The next request tries again.
- The list of databases (`GET /api/dbs`) is still read from the folder on each request, so new files show up without a restart. Each one is loaded on first use.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `rest-api`: adds two requirements. "Databases held in memory": a database is loaded once and then served from memory, and external edits are not seen until restart. "Writes update the in-memory database": later requests see a write without reading the file again, and a failed save is rolled back.

## Impact

- `crates/server/src/api.rs`:
  - `AppState` gains a per-database in-memory cache, replacing the global `write_lock`.
  - The `records`, `get_image` and `write` handlers use the cache instead of `load`/`detect`.
  - `save` is unchanged.
- `crates/server/tests/api.rs`: the `send` helper builds a new app for every request, so existing tests never share a cache. New tests use a single app instance across requests.
- Memory: the server holds each opened database's RGBA image (4 bytes per pixel, 1.6 GB for `big.png`) plus its layout for the whole run.
- Not in scope:
  - Hot reload or watching files for changes.
  - Unloading databases that have not been used for a while, or limiting memory.
  - Loading every database at startup.
  - Making saves faster: a write to `big.png` still re-encodes the whole PNG.
  - CLI and backend behavior.
