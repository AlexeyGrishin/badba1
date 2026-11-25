# Proposal

## Why

badba1's data is meant to be read and edited both in Paint and through an API. The backend can now read and write fields, but only from Rust code. A web server with a REST API makes the databases usable from anywhere, and a small web UI lets a person browse records and edit values, including drawing in image fields, without any tooling.

## What Changes

- New binary crate **`server`** (`crates/server`, axum + tokio).
  - It serves the PNG databases in a folder: `db/` by default, or `--db <dir>`.
  - A database's **id is its file name without `.png`**, for example `db/example1.png` → `example1`.
  - It listens on `127.0.0.1:8080` by default, or `--port <n>`.
- **REST API** (JSON):
  - `GET /api/dbs` lists database ids.
  - `GET /api/dbs/{db}/records?offset&limit` returns a page of records in record order, with the total count. Each record has an **id `"x/y"`**, the top-left of its first field's frame, and its fields in record order as `field_0`, `field_1`, …, each with its type and value. A ref's value is the target record's id or null. Image fields give their size, and their pixels come from a separate endpoint.
  - `GET /api/dbs/{db}/records/{x}/{y}/fields/{i}/image` returns an image field's data area as PNG.
  - `PUT /api/dbs/{db}/records/{x}/{y}/fields/{i}` writes a counter, float, bool or string value from JSON. `PUT …/fields/{i}/image` writes a PNG.
  - Writes go through the backend's `write_field` and are saved to the database file. The response carries the stored value. Write errors such as FieldOverflow and UnknownCharacter come back as JSON errors with fitting HTTP statuses, and leave the file unchanged.
- **Web UI** at `/`: one page served by the same server.
  - Pick a database and page through its records.
  - Each record is shown as in the user's mockup (`ui-mockup.png` in this change): braces, `id: "x/y",`, and one `field_i: …,` line per field.
  - **Editable:** counter (number), **float (number with %)**, bool (checkbox), string (text box) and image (a zoomed canvas: left mouse button draws black, right mouse button erases to the background).
  - Edits are saved right away, and the field then shows the stored value.
  - Refs are shown read-only as `&"x/y"`. The mockup's `column` line is not included, as requested.

## Capabilities

### New Capabilities
- `rest-api`: The HTTP server, database discovery and ids, record and field JSON, record paging, image data, field writes, and error responses.
- `web-ui`: The browser page for viewing and editing records.

### Modified Capabilities
<!-- None. Backend reading and writing rules are used unchanged. -->

## Impact

- New crate `crates/server` (binary `server`) with dependencies `axum 0.8`, `tokio 1`, `serde 1` and `serde_json 1`, added to `[workspace.dependencies]`. Test-only dependencies are `tower` (`util`) and `http-body-util`.
- `backend` is unchanged, apart from possibly a small helper to crop a data area, if useful.
- The web page is a single embedded HTML file (`crates/server/ui/index.html`) with inline CSS and JavaScript, no build step.
- The server modifies files in the database folder it is pointed at. Tests use temporary copies of `testdata/example3.png`.
