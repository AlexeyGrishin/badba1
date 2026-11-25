# Design

## Context

The backend analyzes a PNG with `analyze_png`/`detect`, giving `Layout { fields, links, records }` with each field's `FieldData`. `write_field(image, layout, field_id, WriteValue)` writes one field and returns the read-back value, leaving the image unchanged on error. Records list field ids in record order, and refs give a target record number. See proposal.md for motivation, `specs/rest-api/spec.md` and `specs/web-ui/spec.md` for behavior, and `ui-mockup.png` for the intended look.

Test data: `testdata/example3.png` (the user's `db/example1.png` as of the string-field change; the user's file has since been edited by hand and is not used by tests). Its records are `"35/21"` (F1 image, F2 counter 4, F3 float 43.42%, F5 ref), `"250/23"` (F4 image, F7 bool, F6 bool, F8 string `OPENSPEC`) and `"35/68"` (F9 image, F10 counter 3, F11 ref). Both refs point to `"250/23"`.

## Goals / Non-Goals

**Goals:**
- A thin HTTP layer over the existing backend. Every rule about pixels stays in `backend`.
- A dependency-free single-page UI (plain HTML/CSS/JS embedded in the binary).

**Non-Goals:**
- Authentication, CORS, HTTPS, or binding to interfaces other than localhost.
- Creating, deleting or resizing fields, records or databases. Writing refs.
- Caching, or detecting concurrent edits made in Paint. Every request re-reads the file.

## Decisions

**Crate layout (`crates/server`):**
- `main.rs`: argument parsing (`--db`, `--port`; hand-written like `cli`), folder check, `tokio` runtime, binding to `127.0.0.1`.
- `lib.rs`: `pub fn app(db_dir: PathBuf) -> axum::Router`, so tests can call the router directly with `tower::ServiceExt::oneshot` without a socket.
- `api.rs`: handlers and JSON types.
- `ui/index.html`: served at `/` via `include_str!`.

Dependencies:
- `axum` (0.8, default features), `tokio` (`rt-multi-thread`, `macros`, `net`), `serde` (`derive`) and `serde_json`.
- `image` (workspace) for cropping and encoding data areas.
- Dev-dependencies: `tower` (`util`) and `http-body-util`.

**State:** `AppState { db_dir: PathBuf, write_lock: tokio::sync::Mutex<()> }` in an `Arc`. Reads need no lock. Every write holds the lock for its whole load → write → save sequence, so two writes can't interleave.

**Database access:**
- An id is valid when it matches `^[A-Za-z0-9_-]+$` and `db_dir/{id}.png` exists, otherwise 404. This also rules out path traversal.
- Listing reads the directory and keeps `*.png` files whose stems are valid ids, sorted.
- A request loads the file (`image::open(...).to_rgba8()`) and runs `detect`.
- File work is blocking, so it runs in `tokio::task::spawn_blocking`.

**Ids and addressing:**
- A record's id is `format!("{}/{}", first.outer.left, first.outer.top)`, where `first` is its first field.
- The path `/records/{x}/{y}` finds the record whose first field's frame starts at (x, y).
- `fields/{i}` is the i-th field in that record's order, which maps to a backend field id.
- A ref's target record number maps to that record's id string.

**JSON shapes (serde):**
```json
{"db":"example1","total":3,"offset":0,"limit":20,"background":"#FFFFFF",
 "records":[{"id":"35/21","fields":[
   {"name":"field_0","type":"image","width":33,"height":20},
   {"name":"field_1","type":"counter","value":4},
   {"name":"field_2","type":"float","value":43.42105263157895},
   {"name":"field_3","type":"ref","value":"250/23"}]}]}
```
- Float values are full-precision `percent()`. The UI rounds to 2 decimals for display.
- Write request: `{"value": <json>}`. The handler maps it to `WriteValue` by the field's type:
  - counter needs a non-negative integer
  - float needs a number
  - bool needs a boolean
  - string needs a string

  Anything else is 400 `BadValue`. Image writes take the raw PNG body (`Content-Type: image/png`). An undecodable body is 400 `BadImage`.
- Errors are `{"error": kind, "message": text}`:
  - `WriteError` variants map to their names with 422, except `NoSuchField`, which maps to 404.
  - Unknown db, record or field is 404 (`NotFound`).

**Saving:** encode the modified image as PNG to `{id}.png.tmp` in the same folder, then `rename` it over the original. That way a crash can't leave a half-written database. The backend leaves the image untouched on error, so failed writes skip saving entirely.

**Image endpoint:** crop `inner` from the RGBA image (`image::imageops::crop_imm(...).to_image()`) and encode it as PNG.

**Web UI (`ui/index.html`, vanilla JS):**
- **Layout:** a header with a database `<select>`, prev/next buttons and a limit `<select>` (10/20/50). Records are rendered in a monospace block per the mockup (`{`, indented `id: "x/y",`, `field_i: …,`, `}`).
- **Editors:**
  - counter: `<input type=number min=0 step=1>`
  - float: `<input type=number min=0 max=100 step=0.01>` followed by ` %`
  - bool: checkbox
  - string: `<input type=text>`, uppercased as typed, since the font is uppercase. A value containing `\n` uses a `<textarea>`.
  - ref: text `&"x/y"`
  
  Each editor saves on `change` via `PUT`, then sets its value from the response. On error it shows the message in red beside the field and restores the last stored value.
- **Image:**
  - a `<canvas>` at the data area's pixel size, displayed scaled with CSS `image-rendering: pixelated`. The zoom is `max(2, floor(160 / width))`, so a 33-pixel-wide image is shown 4× and small ones larger.
  - loaded from the image endpoint into the canvas
  - `pointerdown`/`pointermove` with buttons 1/2 sets pixels to black or to the background color (from the records response); `contextmenu` is prevented
  - on `pointerup`, `canvas.toBlob('image/png')` → `PUT …/image`
  - on error, the image is reloaded from the server

**Tests (`crates/server/tests/api.rs`):** each test copies `testdata/example3.png` to a fresh temp folder as `example3.png`, builds `app(dir)` and drives it with `oneshot`. They cover:
- listing (including ignoring non-PNG files)
- unknown and unsafe ids
- record ids and order
- paging (`offset=1&limit=1`, offset past the end)
- field JSON for `"35/21"` and `"250/23"`
- the image endpoint (33×20 PNG)
- writes:
  - float 33.3 → ~33.27, persisted
  - counter 3, persisted (re-analyzed with `backend::analyze_png`)
  - an image upload
- errors:
  - string overflow → 422 with an unchanged file
  - ref → 422 NotWritable
  - unknown record → 404
  - wrong JSON type → 400

Startup errors (a missing folder) are covered by a unit test of the argument and folder check. The UI is checked by hand in a browser during apply: load the page, edit each type, draw and erase, reload.

## Risks / Trade-offs

- [A database edited in Paint while the page is open] → The next request re-reads the file. Field indices may shift if the user changes the layout, so a stale page could write the wrong field. That's acceptable for an experimental tool, and a reload fixes the view.
- [Large images and many records make each request re-run detection] → Fine at this size. A cache keyed on file modification time can come later.
- [Uppercasing text input hides that lowercase is unsupported] → The server still rejects unknown characters, with UnknownCharacter shown in the UI.
- [Paths with `..` or slashes in db ids] → Rejected by the strict id pattern before touching the filesystem.
