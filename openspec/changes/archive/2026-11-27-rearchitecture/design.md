# Design

## Context

See proposal.md (Why) for the motivation. What shapes the approach today:

- `crates/backend` exposes building blocks, not a database: `analyze_image`, `detect`, `write_field`, `bmp::{read_info, read_rect, write_rect}`, the `FieldData` and `WriteValue` enums, mark constants.
- The database itself lives in `crates/server/src/api.rs`: the `Db` struct, `Kind` (PNG/BMP) and `Pixels` (in memory / on disk), `load_db`, record ids, `find_field`, `write_png` (save through a temp file, roll back), `write_bmp` (read a window around the field, write the data area back in place, restore on failure), JSON conversion per type.
- Type knowledge is spread over `types.rs` (marks, float/counter/bool reading), `strings.rs` (text reading), `references.rs` (ref lines, done after records are known), `write.rs` (per-type write plans), `layout.rs` (`FieldData`), `crates/cli/src/main.rs` (`data()` text per type), and `api.rs` (`field_json`, `to_write_value`).
- Format knowledge is spread over `load.rs` (`load_png`, `is_bmp`, `load_image`, error variants per format), `bmp.rs`, and `api.rs`.
- `ref` values need a second pass: they can only be resolved after all fields and records are known, and they read the whole image (lines outside fields).
- Only the CLI, the server and this repository's tests use the library, so its Rust API can change freely. Files, CLI output, REST responses and the web UI must not change.

## Goals / Non-Goals

**Goals:**
- One documented entry point, `load_db` → `Database`, that the CLI and server use for everything.
- A field type is one module plus one registry line; a storage format is one module plus one registry line.
- Byte-identical CLI output, REST responses and file writes before and after.

**Non-Goals:**
- Renaming the `backend` crate, adding formats or types, or changing any behavior.
- Making the web UI generic: its editors still switch on the type name from the JSON (outside the field-types module rule).
- Runtime plugins (dynamic loading, registering types from outside the crate). Extension is at compile time, by adding a module.
- Thread-safety beyond `Database: Send + Sync`; the server keeps its own per-database `RwLock` cache.

## Decisions

### 1. Module layout

```
crates/backend/src/
  lib.rs            crate docs + example, re-exports of the public API
  db.rs             Database facade: load_db, records, field, field_image, set_field
  error.rs          Error (load / not found / write / save)
  layout.rs         Rect, Borders, Field, Link, Record, Layout (geometry only)
  detect.rs         fields and links (unchanged algorithm), then asks the type registry
  records.rs        record grouping and order (unchanged)
  field_map.rs      FieldMap (moved out of references.rs; used by links and ref lines)
  font.rs           glyph font (shared resource, unchanged)
  draw.rs           generic write: clear data area, draw a plan, no per-type code
  types/mod.rs      FieldType trait, Value, Input, Mark, mark_area, TYPES registry, recognize()
  types/image.rs  types/float.rs  types/counter.rs  types/bool.rs
  types/reference.rs (the `ref` type; `ref` is a Rust keyword)
  types/string/{mod.rs, read.rs, write.rs}   (text reading from strings.rs, writing from write.rs)
  formats/mod.rs    Format and Storage traits, FORMATS registry, format_for(path), size check
  formats/png.rs  formats/bmp.rs
```

`types.rs`, `strings.rs`, `references.rs`, `write.rs`, `load.rs` and `bmp.rs` disappear; their code and unit tests move with it. A type may be a folder module when it is large (`string`); the rule is one module per type, not one file.

### 2. Field types: a trait plus a static registry

```rust
pub trait FieldType: Sync {
    fn name(&self) -> &'static str;
    fn mark(&self) -> Option<Mark>;                       // None only for `image`
    fn read(&self, area: &AreaView, color: Rgba<u8>) -> Value;   // from the data area alone
    fn resolve(&self, ctx: &ResolveCtx, field: &mut Field) {}    // optional second pass (`ref`)
    fn input_kind(&self) -> Option<InputKind>;            // None = not writable
    fn plan(&self, field: &Field, color: Option<Rgba<u8>>, input: Input) -> Result<Plan, WriteError>;
    fn cli_text(&self, value: &Value) -> String;
}
pub static TYPES: &[&dyn FieldType] = &[&Image, &Float, &Counter, &Bool, &Reference, &Str];
```

- Recognition: `types::recognize` reads the mark color and shape exactly as today, then looks the shape up in `TYPES`; no match → `image`. A unit test checks shapes are unique and include the top-middle pixel (spec: Distinct mark shapes).
- `AreaView` is the data area plus the background color, over either the whole image (detection) or a small bitmap (read-back after a write). Types that only look at their data area therefore never need the whole image, which is what lets a BMP write read its value back from the new data-area pixels alone.
- `ref` needs the whole image and the records: detection calls `resolve` on every field after records are built, with the image, fields, records and `FieldMap` in `ResolveCtx`. Only `Reference` overrides it. This keeps the two-pass behavior and the memory order of `detect` (mask dropped before refs are traced).
- Alternatives: keeping the `FieldData` enum with per-variant modules (adding a type would still touch the enum and every `match` on it); a trait object per field value (`Box<dyn FieldValue>`) (clients would need downcasts to show values). The registry plus generic `Value` keeps clients type-agnostic.

### 3. Generic values instead of `FieldData` / `WriteValue`

```rust
pub enum Value { Bitmap { width: u32, height: u32 }, Int(u64), Number(f64),
                 Bool(bool), Text(String), Record(Option<usize>) }
pub enum Input { Int(u64), Number(f64), Bool(bool), Text(String), Bitmap(RgbaImage) }
```

- A `Field` carries `kind: &'static dyn FieldType`, `color: Option<Rgba<u8>>` and `value: Value`. `Record(n)` is a record number; the database turns it into an id (`"x/y"`).
- The set of kinds is closed on purpose: it is what clients switch on (server JSON, CLI fallbacks), so a new type that reuses a kind needs no client change. A type that needs a genuinely new kind is a deliberate API change.
- `float` keeps its exact value as `Number(percent)` computed from `filled/total` exactly as `percent()` does today, so CLI (`{:.2}%`) and JSON output stay identical. Because of `f64`, `Field` and `Layout` drop `Eq`/`Hash` and keep `PartialEq`; only tests compare them.
- Type mismatch messages stay identical: the "value type" in `TypeMismatch` is the name of the registered type whose `input_kind` matches the input's kind (`Text` → `string`, `Int` → `counter`, …), so "field F7 is a counter field, not a string field" is unchanged.
- The server converts JSON by the field's `input_kind` (`Int` → `as_u64`, `Number` → `as_f64`, `Bool`, `Text`; `Bitmap` → the existing "fields are not written with a JSON value" error; `None` → `NotWritable`, checked first as today) and converts `Value` to JSON by kind. The CLI prints `type.cli_text(value)` (`Record` → `ref R2` / `ref null`).

### 4. Storage formats: a trait plus a static registry

```rust
pub trait Format: Sync {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn dimensions(&self, path: &Path) -> Result<(u32, u32), Error>;     // header only
    fn open(&self, path: &Path) -> Result<(RgbaImage, Box<dyn Storage>), Error>;
}
pub trait Storage: Send + Sync {
    fn read_rect(&self, rect: Rect) -> Result<RgbaImage, Error>;
    fn stored(&self, bitmap: &mut RgbaImage);       // what the file can hold (BMP w/o alpha → opaque)
    fn write_rect(&mut self, rect: Rect, pixels: &RgbaImage) -> Result<(), Error>; // all-or-nothing
}
pub static FORMATS: &[&dyn Format] = &[&Png, &Bmp];
```

- `format_for(path)` picks the format owning the extension (any case), else PNG (spec: Format chosen by extension). `formats::open` checks `dimensions` against `MAX_PIXELS` before `open` decodes anything, so the size rule is in one place for all formats.
- `open` returns the decoded image for detection plus the storage. PNG storage keeps the image; BMP storage drops it and keeps `BmpInfo` (as the server does today), reading rows from the file.
- `write_rect` is all-or-nothing: PNG copies the rectangle in, saves via temp file + rename, and copies the old pixels back on failure; BMP writes the rows in place and writes the old bytes back on failure. This is today's `write_png` / `write_bmp` logic, moved out of the server.
- Error variants become format-neutral (`Decode { path, format, reason }`, `Unsupported { path, format, reason }`) while the formatted messages stay exactly as today ("cannot decode X as PNG: …", "unsupported BMP format: …").
- The server's directory listing and ambiguity check iterate `FORMATS` extensions instead of hard-coding `png`/`bmp`.

### 5. The `Database` facade owns writes

`Database::set_field(record, index, input)`:
1. Find the field (not-found errors with today's texts: "no record x/y", "record x/y has no field i").
2. If its type is not writable → `NotWritable` (before looking at the input, as today).
3. For a bitmap input, apply `storage.stored(&mut bitmap)`; then `type.plan(...)` validates and plans (no pixels touched).
4. Build the new data area: background-filled bitmap of the data area's size, plan drawn on it (`draw.rs`).
5. `storage.write_rect(field.inner, &new_area)`; on error return a save error, memory unchanged.
6. Read the value back with `type.read(AreaView::of(&new_area, background), color)` and store it in the field.

This replaces the BMP "window" trick: since types read from the data area and background only, no window around the frame and mark is needed. The PNG path produces the same pixels as today's `write_field` (same clear-then-draw order), so files stay byte-identical. `write_field(image, layout, ...)` on an in-memory image remains as a lower-level public function (used by tests), built on the same `draw` code.

Naming: the user-facing names follow Rust conventions (`records`, `record`, `field`, `field_image`, `set_field`) with `#[doc(alias = "get_records")]`, `#[doc(alias = "get_field")]` so the requested names find them in rustdoc; `load_db` is a free function and `Database::load` an equivalent constructor.

### 6. Documentation

`#![warn(missing_docs)]` in `lib.rs`, plus a CI-style test step `RUSTDOCFLAGS="-D missing_docs" cargo doc -p backend`. The crate docs get an overview (database = image, fields, records, types, formats), a "How to add a type" and "How to add a format" section pointing at the registries, and a runnable example that copies `testdata/example3.png` to a temp dir, loads it, reads a field and writes one.

### 7. Behavior lock before refactoring

Before moving code, capture golden outputs from the current build: `cli` output for every `testdata` image and its BMP copies, and REST JSON for records pages, field images and write responses of the example databases. Tests compare against these goldens through every step, so "no behavior change" is checked, not assumed.

## Risks / Trade-offs

- [Big-bang refactor breaks something subtle (ordering, error text, float rounding)] → goldens from step 7 first; move code in small steps (formats, then types, then facade, then clients), keeping all tests green after each.
- [Dynamic dispatch on every field read slows large images] → one virtual call per field, negligible next to pixel scans; `large_image` test and the 100 ms BMP scenario stay as guards.
- [Generic `Value` loses the float's `filled/total`] → only the percentage was ever shown; tests that checked `filled` move into `types/float.rs` and use the type's read function directly.
- [BMP write without the frame/mark window changes read-back] → types read only data area + background (mark color is passed in), which is exactly what the window provided; covered by the existing BMP write tests and goldens.
- [Archive leaves `openspec/specs/strings` and `openspec/specs/references` with no requirements] → delete both folders right after archiving (task), since their content now lives in `types/string` and `types/ref`.
- [`Layout` loses `Eq`] → only `PartialEq` was used outside tests.

## Migration Plan

Internal refactor, done in order on this branch: goldens → formats → types and values → `Database` facade → server and CLI onto the facade → docs → spec cleanup. Each step compiles and passes `cargo test` at the workspace root. Rollback is reverting the branch; no file format, data or API consumer outside this repository is affected.
