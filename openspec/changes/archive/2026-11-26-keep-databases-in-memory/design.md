# Design

## Context

`crates/server/src/api.rs` today:
- `AppState { db_dir, write_lock: Mutex<()> }`. The single global lock means a write to any database blocks every other write.
- `records` and `get_image` each run `db_path` → `load` (decode the PNG) → `detect`.
- `write` holds the global lock while it runs `load` → `detect` → `find_field` → `write_field` → `save` (a temporary file, then a rename).
- All image work runs in `spawn_blocking`.

Backend facts this design relies on:
- `write_field(image, layout, id, value)` validates the value first (`plan`) and only then changes pixels. It touches only `field.inner`, and it returns the field's new `FieldData`.
- Pixels written in frame color are rejected (`FrameColor`). So a write cannot add or remove fields, links or records, and cannot change a type mark or a ref line, all of which lie outside data areas. After a write, only that one field's `data` changes.

Measured on `big.png` (release): load plus detect takes about 8–13 s; the image takes 1.6 GB in memory.

## Goals / Non-Goals

**Goals:**
- Each database is decoded and analyzed at most once per server run, unless loading it fails.
- Reads of different databases, and concurrent reads of one database, run in parallel.
- After every successful write, the file on disk and the copy in memory hold the same data.

**Non-Goals:**
- Noticing external edits to the file (no file watching and no modification-time checks).
- Evicting databases or capping memory.
- Writing in the background, or batching several writes into one save.

## Decisions

### 1. Load lazily, on first use, and keep the database for the whole run
A new `Db` struct holds `{ image: RgbaImage, layout: Layout, ids: Vec<String> }`, where `ids` is the record ids from `record_ids`. It is created the first time a handler names that database.
- *Alternative:* load every database when the server starts. This was rejected because startup would then take as long as loading every database (about 13 s for `big.png`), and databases that nobody opens would still use memory.

### 2. Cache structure: one map entry per database, each with its own lock
- **Map:** `AppState.dbs: std::sync::Mutex<HashMap<String, Arc<RwLock<Option<Db>>>>>`. The map mutex is held only long enough to look up or insert an entry, never during a load.
- **Lock:** each entry has its own `std::sync::RwLock`. The locks are taken inside `spawn_blocking`, which matches the existing `blocking` helper.
- **Read flow:** take the read lock. If the entry is `Some`, use it. If it is `None`, release the read lock, take the write lock, and check again. If it is still `None`, call `db_path` and `load`, then `detect`, then store the result.
  - Because the second check happens under the write lock, concurrent first requests trigger only one load.
  - If the load fails, the entry is left `None` and the error is returned. The next request tries again.
- **Lookup order:** if the entry is already `Some`, `db_path` is not checked. This is what lets a cached database keep being served after its file is deleted.
- **Id check:** `valid_id` is still checked before the map is touched, so an invalid id never creates an entry.
- *Alternative:* `tokio::sync::RwLock` held across `.await`. This was rejected because all the real work is already blocking code in `spawn_blocking`, so a std lock inside it is simpler.
- *Alternative:* `OnceLock<Db>` for each entry. This was rejected because writes need mutable access and failed loads must not be remembered.

### 3. A write updates memory in place, then saves; it does not re-run detection
The write flow holds the entry's write lock for all of these steps:
1. Load the database if it is not yet in memory.
2. Run `find_field`, and reject a `ref` field.
3. Run `make(field)`.
4. Copy the field's current data-area pixels with `crop_imm(inner)`. This is a small buffer.
5. Run `write_field`, which returns `stored`.
6. Set `layout.fields[id-1].data` to `stored`.
7. Run `save`.

If `save` fails, the copied pixels are written back into the image (`copy_from` at `inner`), the old `data` is restored, and the error is returned. Running detection again after each write would cost about 8 s on `big.png`. That is unnecessary, because of the backend facts listed under Context.
- *Alternative:* save from a clone, outside the lock. This was rejected because the clone costs 1.6 GB for `big.png`, and readers would see a value that has not yet been saved.
- *Trade-off:* reads of that database wait for the save to finish, which takes seconds for `big.png`. Other databases are not affected.

### 4. Remove the global `write_lock`
Each entry's `RwLock` now serializes writes to its own database. Writes to different databases no longer block each other.

### 5. Listing stays on disk
`list_dbs` still reads the folder on each request. A file that appears later is loaded the first time it is requested. If a file is removed, the database stays listed only while its file exists on disk, but if it was already loaded it can still be served from memory. This is acceptable, because external changes are explicitly unsupported in this change.

### 6. Test strategy
Today the tests' `send` helper builds a new app for each request. To test caching, a helper builds one `Router` and sends several requests to it with `ServiceExt::oneshot(router.clone())`; `Router` is `Clone`, and clones share the `Arc` state. Two kinds of tests use it:
- **Caching:** the test replaces or deletes the file between requests, and checks that the responses come from memory.
- **Failed save:** the test creates a *directory* named `example3.png.tmp`, so that `save` fails.

## Risks / Trade-offs

- [Memory grows with every database opened and is never freed] → This is accepted for an experimental server and is stated in the proposal. An LRU cache could be added later.
- [Edits made in MS Paint while the server is running are invisible until restart, and the next API write will overwrite them] → This is documented as a breaking behavior change. Hot reload was ruled out by the request. The module doc comment in `api.rs` is updated to say so.
- [A write to `big.png` blocks reads of `big.png` for several seconds while it saves] → This is acceptable for now. Writing in the background is a non-goal.
- [A poisoned `RwLock` after a panic inside a handler] → `PoisonError::into_inner` is used to recover the guard, because the data is still consistent: a write changes memory only after validation, and rolls back if the save fails.
