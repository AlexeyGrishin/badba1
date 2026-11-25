# Tasks

## 1. Server crate

- [x] 1.1 Add `axum = "0.8"`, `tokio = { version = "1", features = ["rt-multi-thread", "macros", "net"] }`, `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"` to `[workspace.dependencies]`. Create `crates/server` (binary `server`, depending on `backend`, `image` and those crates, with dev-dependencies `tower` (`util`) and `http-body-util`), with `lib.rs` exposing `app(db_dir) -> Router` and `main.rs` parsing `--db`/`--port`, checking the folder and serving on `127.0.0.1`. Verify `cargo build -p server` succeeds, and a unit test shows a missing folder gives an error naming it.

## 2. REST API

- [x] 2.1 Implement `GET /api/dbs` (valid-id `.png` stems, sorted) and the id check (`^[A-Za-z0-9_-]+$` plus file exists, else 404 JSON). Verify API tests: listing returns `["example3"]` with a `notes.txt` present and ignored, and `missing` and `..%2Fsecret` give 404.
- [x] 2.2 Implement `GET /api/dbs/{db}/records?offset&limit` (defaults 0/20, max 1000, `total`, `background` as `#RRGGBB`), with record ids `"x/y"` and field JSON per type (refs as target ids). Verify API tests: ids `"35/21"`, `"250/23"`, `"35/68"` in order; `offset=1&limit=1` gives `"250/23"` and total 3; offset 10 gives an empty list; `"35/21"` fields are image 33×20, counter 4, float ≈43.42 and ref `"250/23"`; and `"250/23"`'s last field is string `"OPENSPEC"`.
- [x] 2.3 Implement `GET …/records/{x}/{y}/fields/{i}/image` (the cropped data area as PNG, 404 for non-image fields). Verify an API test: field 0 of `"35/21"` returns a 33×20 `image/png` equal to the data area.
- [x] 2.4 Implement `PUT …/fields/{i}` (JSON value mapped by field type, 400 `BadValue` on a mismatch) and `PUT …/fields/{i}/image` (PNG body, 400 `BadImage`), using a write lock, `write_field`, and saving via a temp file plus rename. Map `WriteError` to 422 (`NoSuchField` to 404). Verify API tests:
  - float 33.3 → ≈33.27, also in a later GET
  - counter 3, confirmed by `backend::analyze_png` on the file
  - a 10×10 image upload whose top-left pixels match
  - `"HELLO WORLD"` to the string field → 422 FieldOverflow with the file bytes unchanged
  - a ref field → 422 NotWritable
  - `/records/1/1/fields/0` → 404
  - `"abc"` to a counter → 400

## 3. Web UI

- [x] 3.1 Add `crates/server/ui/index.html` (embedded, served at `/`): the database selector, prev/next and limit controls, records in the mockup layout, editors per type (number, number + %, checkbox, text, read-only `&"x/y"`), saving on change, showing stored values, and inline errors that revert to the stored value. Verify an API test shows `GET /` returns HTML, and check by hand in a browser against a temporary copy of the reference image that records render as in the mockup, and that counter, float (33.3 → 33.27), bool and string edits save and survive a reload.
- [x] 3.2 Add image editing: a zoomed pixel canvas loaded from the image endpoint; LMB draws black, RMB erases to the background (no context menu); saving on release via the image PUT. Verify by hand in a browser that drawing and erasing save and survive a reload.

## 4. Workspace checks

- [x] 4.1 Run `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Verify all succeed with no warnings.

## Workflow follow-up

- Archive with `/openspec-archive-change`, then commit everything in one commit (per the project's archive guidance). The mockup `ui-mockup.png` is archived with the change.
