# Tasks

## 1. In-memory database cache

- [x] 1.1 In `crates/server/src/api.rs`, add `struct Db { image, layout, ids }` and `AppState.dbs` (a map from id to `Arc<RwLock<Option<Db>>>`), and remove `write_lock`. Update the module doc comment to say that databases are loaded once and external edits need a restart. Verify with `cargo build -p server`.
- [x] 1.2 Add `with_db(state, id, |&Db| …)` for reads and `with_db_mut(state, id, |&mut Db| …)` for writes, as in design Decision 2. They check `valid_id` first, double-check under the write lock, load with `db_path` + `load` + `detect` + `record_ids`, never cache a failed load, and recover from lock poisoning. Verify with `cargo build -p server`.
- [x] 1.3 Switch `records` and `get_image` to `with_db`. Verify that `cargo test -p server` passes unchanged.

## 2. Writes in memory

- [x] 2.1 Rewrite `write` to use `with_db_mut`. Copy the data area, call `write_field`, set the field's `data` to the stored value, then `save`. On a failed save, put back the copied pixels and the old `data` (design Decision 3). Do not re-run detection. Verify that `cargo test -p server` passes, including `counter_write_persists_to_file`, `overflow_is_422_and_file_unchanged` and `image_upload`.

## 3. Tests for in-memory behavior

- [x] 3.1 Add a test helper that sends several requests to one `Router` instance (design Decision 6). Verify that it compiles and that an existing test rewritten to use it still passes.
- [x] 3.2 Add tests for the spec's "Databases held in memory" scenarios:
  - Records stay the same after the file is replaced on disk.
  - The image endpoint still works after the file is deleted.
  - A failed load is retried once the file is fixed.
  - A new file is listed and served.

  Verify with `cargo test -p server`.
- [x] 3.3 Add tests for the spec's "Writes update the in-memory database" scenarios:
  - After a write, replace the file on disk; the records page still shows the written value.
  - With a directory named `example3.png.tmp`, a write returns 500, and the records page shows the old value.

  Verify with `cargo test -p server`.

## 4. Large-database check

- [x] 4.1 Start a release `server --db db --port 8099` and time three requests. The first `GET /api/dbs/big/records` takes the full load time (about 8–13 s). A second records request and a field image request each finish in under 1 s. Verify by recording the times, then stop only that server process.

## 5. Final check

- [x] 5.1 Run `cargo test --workspace` and `cargo clippy --workspace --all-targets`. Verify that everything passes with no warnings.

## Workflow follow-up

- Archive the change after review, then commit code and `openspec/` together in one commit named after the change.
