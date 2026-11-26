//! REST API handlers.
//!
//! A database is read and analyzed once, when a request first names it, and is
//! then served from memory for the rest of the run. Changes made to the file by
//! other programs are not seen until the server restarts. Each database has its
//! own read/write lock: reads run in parallel, and a write updates memory and
//! then saves through a temporary file and a rename, so a database is never
//! half-written. A write whose save fails is undone in memory.
//!
//! Databases are `.png` or `.bmp` files. A PNG's pixels stay in memory. A BMP
//! is decoded once for detection and then dropped: only its layout and
//! background color stay in memory, field images are read from the file's
//! rows, and a write overwrites just the field's data area in place.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, RwLock};

use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use backend::bmp::BmpInfo;
use backend::{
    Field, FieldData, Layout, Rect, WriteError, WriteValue, detect, mark_area, write_field,
};
use image::{GenericImage, Rgba, RgbaImage};
use serde::Deserialize;
use serde_json::{Value, json};

/// The web page, served at `/`.
const INDEX_HTML: &str = include_str!("../ui/index.html");

/// Default and maximum page sizes for the records endpoint.
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 1000;

struct AppState {
    db_dir: PathBuf,
    /// Databases by id. An entry holds `None` until its first successful load.
    dbs: Mutex<HashMap<String, Slot>>,
}

/// A database held in memory.
struct Db {
    /// The file it was loaded from, and is saved to.
    path: PathBuf,
    pixels: Pixels,
    layout: Layout,
    /// Record ids, indexed by record number (see [`record_ids`]).
    ids: Vec<String>,
    /// The image's top-left pixel, which counts as the background.
    background: Rgba<u8>,
}

/// The file formats a database can have.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Png,
    Bmp,
}

/// Where a database's pixels are.
enum Pixels {
    /// A PNG's pixels, decoded and kept in memory.
    Memory(RgbaImage),
    /// A BMP's pixels, read and written in place in its file.
    Bmp(BmpInfo),
}

type Slot = Arc<RwLock<Option<Db>>>;

impl AppState {
    /// The slot of database `id`. A slot is created only for a valid id whose
    /// file exists; once created it stays, even if the file goes away.
    fn slot(&self, id: &str) -> ApiResult<Slot> {
        let mut dbs = self.dbs.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(slot) = dbs.get(id) {
            return Ok(slot.clone());
        }
        db_path(&self.db_dir, id)?;
        Ok(dbs.entry(id.to_string()).or_default().clone())
    }
}

/// Reads and analyzes database `id` from its file. A BMP's decoded image is
/// dropped once the layout is known.
fn load_db(dir: &Path, id: &str) -> ApiResult<Db> {
    let (path, kind) = db_path(dir, id)?;
    let image = match kind {
        Kind::Png => backend::load_png(&path),
        Kind::Bmp => backend::bmp::load_bmp(&path),
    }
    .map_err(load_error)?;
    let layout = detect(&image);
    let ids = record_ids(&layout);
    let background = *image.get_pixel(0, 0);
    let pixels = match kind {
        Kind::Png => Pixels::Memory(image),
        Kind::Bmp => {
            drop(image);
            Pixels::Bmp(backend::bmp::read_info(&path).map_err(load_error)?)
        }
    };
    Ok(Db {
        path,
        pixels,
        layout,
        ids,
        background,
    })
}

/// Runs `f` on database `id`, loading it first if this is its first use.
/// Concurrent first uses load it once; a failed load is not remembered.
fn with_db<T>(state: &AppState, id: &str, f: impl FnOnce(&Db) -> ApiResult<T>) -> ApiResult<T> {
    let slot = state.slot(id)?;
    {
        let guard = slot.read().unwrap_or_else(PoisonError::into_inner);
        if let Some(db) = guard.as_ref() {
            return f(db);
        }
    }
    with_db_mut(state, id, |db| f(db))
}

/// Like [`with_db`], with exclusive access for writing.
fn with_db_mut<T>(
    state: &AppState,
    id: &str,
    f: impl FnOnce(&mut Db) -> ApiResult<T>,
) -> ApiResult<T> {
    let slot = state.slot(id)?;
    let mut guard = slot.write().unwrap_or_else(PoisonError::into_inner);
    if guard.is_none() {
        *guard = Some(load_db(&state.db_dir, id)?);
    }
    f(guard.as_mut().expect("loaded above"))
}

type Shared = Arc<AppState>;

/// The server's routes, serving the databases in `db_dir`.
pub fn app(db_dir: PathBuf) -> Router {
    let state = Arc::new(AppState {
        db_dir,
        dbs: Mutex::new(HashMap::new()),
    });
    Router::new()
        .route("/", get(index))
        .route("/api/dbs", get(list_dbs))
        .route("/api/dbs/{db}/records", get(records))
        .route(
            "/api/dbs/{db}/records/{x}/{y}/fields/{i}",
            axum::routing::put(put_value),
        )
        .route(
            "/api/dbs/{db}/records/{x}/{y}/fields/{i}/image",
            get(get_image).put(put_image),
        )
        .with_state(state)
}

/// An error response: `{"error": kind, "message": text}` with a status.
#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    kind: &'static str,
    message: String,
}

impl ApiError {
    fn not_found(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            kind: "NotFound",
            message: message.into(),
        }
    }

    fn bad_request(kind: &'static str, message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            kind,
            message: message.into(),
        }
    }

    fn conflict(kind: &'static str, message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::CONFLICT,
            kind,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            kind: "Internal",
            message: message.into(),
        }
    }
}

impl From<WriteError> for ApiError {
    fn from(e: WriteError) -> Self {
        let kind = match e {
            WriteError::NoSuchField(_) => return ApiError::not_found(e.to_string()),
            WriteError::NotWritable { .. } => "NotWritable",
            WriteError::TypeMismatch { .. } => "TypeMismatch",
            WriteError::FieldOverflow { .. } => "FieldOverflow",
            WriteError::OutOfRange { .. } => "OutOfRange",
            WriteError::UnknownCharacter { .. } => "UnknownCharacter",
            WriteError::FrameColor { .. } => "FrameColor",
        };
        ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            kind,
            message: e.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.kind, "message": self.message }));
        (self.status, body).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

/// Runs blocking file and image work off the async runtime.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> ApiResult<T> + Send + 'static,
) -> ApiResult<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
}

/// Whether `id` is a valid database id: letters, digits, `-` and `_`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The file and format of database `id`: `<id>.png` or `<id>.bmp`. 404 if
/// the id is invalid or neither file exists, 409 if both do.
fn db_path(dir: &Path, id: &str) -> ApiResult<(PathBuf, Kind)> {
    let not_found = || ApiError::not_found(format!("no database {id:?}"));
    if !valid_id(id) {
        return Err(not_found());
    }
    let png = dir.join(format!("{id}.png"));
    let bmp = dir.join(format!("{id}.bmp"));
    match (png.is_file(), bmp.is_file()) {
        (true, true) => Err(ApiError::conflict(
            "AmbiguousDatabase",
            format!("database {id:?} exists as both {id}.png and {id}.bmp"),
        )),
        (true, false) => Ok((png, Kind::Png)),
        (false, true) => Ok((bmp, Kind::Bmp)),
        (false, false) => Err(not_found()),
    }
}

fn load_error(e: backend::Error) -> ApiError {
    match e {
        backend::Error::TooLarge { .. } => ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            kind: "ImageTooLarge",
            message: e.to_string(),
        },
        _ => ApiError::internal(e.to_string()),
    }
}

/// Reads rectangle `rect` of a BMP database's file.
fn read_bmp_rect(path: &Path, info: &BmpInfo, rect: Rect) -> ApiResult<RgbaImage> {
    let fail =
        |e: std::io::Error| ApiError::internal(format!("cannot read {}: {e}", path.display()));
    let mut file = File::open(path).map_err(fail)?;
    backend::bmp::read_rect(&mut file, info, rect).map_err(fail)
}

/// Saves `image` to `path` via a temporary file in the same folder.
fn save(path: &Path, image: &RgbaImage) -> ApiResult<()> {
    let tmp = path.with_extension("png.tmp");
    let fail = |e: &dyn std::fmt::Display| ApiError::internal(format!("cannot save: {e}"));
    image
        .save_with_format(&tmp, image::ImageFormat::Png)
        .map_err(|e| fail(&e))?;
    std::fs::rename(&tmp, path).map_err(|e| fail(&e))
}

fn png_bytes(image: &RgbaImage) -> ApiResult<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    image
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(out.into_inner())
}

/// Record ids (`"x/y"` of the first field's frame), indexed by record number.
fn record_ids(layout: &Layout) -> Vec<String> {
    let mut ids = vec![String::new(); layout.records.len() + 1];
    for r in &layout.records {
        let first = &layout.fields[r.fields[0] - 1];
        ids[r.id] = format!("{}/{}", first.outer.left, first.outer.top);
    }
    ids
}

/// The JSON of a field with the given name and data.
fn field_json(name: String, field: &Field, data: &FieldData, ids: &[String]) -> Value {
    let mut v = json!({ "name": name, "type": data.type_name() });
    let value = match data {
        FieldData::Image => {
            v["width"] = json!(field.inner.width());
            v["height"] = json!(field.inner.height());
            return v;
        }
        FieldData::Counter { count, .. } => json!(count),
        FieldData::Float { .. } => json!(data.percent()),
        FieldData::Bool { value, .. } => json!(value),
        FieldData::String { text, .. } => json!(text),
        FieldData::Ref { record, .. } => json!(record.map(|r| ids[r].clone())),
    };
    v["value"] = value;
    v
}

/// The field (and its index in the record) at record `x/y`, field index `i`.
fn find_field(layout: &Layout, x: u32, y: u32, i: usize) -> ApiResult<&Field> {
    let record = layout
        .records
        .iter()
        .find(|r| {
            let first = &layout.fields[r.fields[0] - 1];
            first.outer.left == x && first.outer.top == y
        })
        .ok_or_else(|| ApiError::not_found(format!("no record {x}/{y}")))?;
    let id = record
        .fields
        .get(i)
        .ok_or_else(|| ApiError::not_found(format!("record {x}/{y} has no field {i}")))?;
    Ok(&layout.fields[id - 1])
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn list_dbs(State(state): State<Shared>) -> ApiResult<Json<Value>> {
    let dir = state.db_dir.clone();
    blocking(move || {
        let entries = std::fs::read_dir(&dir).map_err(|e| ApiError::internal(e.to_string()))?;
        let mut dbs: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "png" || x == "bmp"))
            .filter_map(|p| p.file_stem()?.to_str().map(str::to_string))
            .filter(|id| valid_id(id))
            .collect();
        dbs.sort();
        dbs.dedup();
        Ok(Json(json!({ "dbs": dbs })))
    })
    .await
}

#[derive(Deserialize)]
struct Page {
    offset: Option<usize>,
    limit: Option<usize>,
}

async fn records(
    State(state): State<Shared>,
    UrlPath(db): UrlPath<String>,
    Query(page): Query<Page>,
) -> ApiResult<Json<Value>> {
    blocking(move || with_db(&state, &db, |d| records_page(&db, d, &page))).await
}

/// The JSON of one page of records of database `db`.
fn records_page(db: &str, d: &Db, page: &Page) -> ApiResult<Json<Value>> {
    let (layout, ids) = (&d.layout, &d.ids);
    let offset = page.offset.unwrap_or(0);
    let limit = page.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let records: Vec<Value> = layout
        .records
        .iter()
        .skip(offset)
        .take(limit)
        .map(|r| {
            let fields: Vec<Value> = r
                .fields
                .iter()
                .enumerate()
                .map(|(i, &id)| {
                    let f = &layout.fields[id - 1];
                    field_json(format!("field_{i}"), f, &f.data, ids)
                })
                .collect();
            json!({ "id": ids[r.id], "fields": fields })
        })
        .collect();
    let bg = d.background.0;
    Ok(Json(json!({
        "db": db,
        "total": layout.records.len(),
        "offset": offset,
        "limit": limit,
        "background": format!("#{:02X}{:02X}{:02X}", bg[0], bg[1], bg[2]),
        "records": records,
    })))
}

async fn get_image(
    State(state): State<Shared>,
    UrlPath((db, x, y, i)): UrlPath<(String, u32, u32, usize)>,
) -> ApiResult<Response> {
    blocking(move || {
        with_db(&state, &db, |d| {
            let f = find_field(&d.layout, x, y, i)?;
            if f.data != FieldData::Image {
                return Err(ApiError::not_found(format!(
                    "field {i} of record {x}/{y} is not an image field"
                )));
            }
            let r = f.inner;
            let crop = match &d.pixels {
                Pixels::Memory(image) => {
                    image::imageops::crop_imm(image, r.left, r.top, r.width(), r.height())
                        .to_image()
                }
                Pixels::Bmp(info) => read_bmp_rect(&d.path, info, r)?,
            };
            let bytes = png_bytes(&crop)?;
            Ok(([(header::CONTENT_TYPE, "image/png")], bytes).into_response())
        })
    })
    .await
}

#[derive(Deserialize)]
struct ValueBody {
    value: Value,
}

/// Converts a JSON value to a write value for a field of type `type_name`.
fn to_write_value(type_name: &str, v: &Value) -> ApiResult<WriteValue> {
    let bad = || {
        ApiError::bad_request(
            "BadValue",
            format!("{v} is not a valid value for a {type_name} field"),
        )
    };
    Ok(match type_name {
        "counter" => WriteValue::Counter(v.as_u64().ok_or_else(bad)?),
        "float" => WriteValue::Float(v.as_f64().ok_or_else(bad)?),
        "bool" => WriteValue::Bool(v.as_bool().ok_or_else(bad)?),
        "string" => WriteValue::String(v.as_str().ok_or_else(bad)?.to_string()),
        _ => {
            return Err(ApiError::bad_request(
                "BadValue",
                format!("{type_name} fields are not written with a JSON value"),
            ));
        }
    })
}

/// Writes the value built by `make` into field `i` of record `x/y` of database
/// `db` in memory, saves the file, and returns the field's JSON. A write only
/// changes pixels inside the field's data area (frame color is rejected), so
/// the layout stays valid and only that field's data is updated. If the save
/// fails, the data area's previous pixels are put back.
async fn write(
    state: Shared,
    db: String,
    (x, y, i): (u32, u32, usize),
    make: impl FnOnce(&Field) -> ApiResult<WriteValue> + Send + 'static,
) -> ApiResult<Json<Value>> {
    blocking(move || {
        with_db_mut(&state, &db, |d| {
            let field = find_field(&d.layout, x, y, i)?.clone();
            if let FieldData::Ref { .. } = field.data {
                return Err(WriteError::NotWritable {
                    field: field.id,
                    type_name: "ref",
                }
                .into());
            }
            let value = make(&field)?;
            let stored = match &mut d.pixels {
                Pixels::Memory(image) => write_png(&d.path, image, &d.layout, &field, value)?,
                Pixels::Bmp(info) => write_bmp(&d.path, info, d.background, &field, value)?,
            };
            d.layout.fields[field.id - 1].data = stored.clone();
            Ok(Json(field_json(
                format!("field_{i}"),
                &field,
                &stored,
                &d.ids,
            )))
        })
    })
    .await
}

/// Writes `value` into `field` of a PNG database held in memory as `image`,
/// then saves the whole file. If the save fails, the data area's previous
/// pixels are put back.
fn write_png(
    path: &Path,
    image: &mut RgbaImage,
    layout: &Layout,
    field: &Field,
    value: WriteValue,
) -> ApiResult<FieldData> {
    let r = field.inner;
    let before = image::imageops::crop_imm(image, r.left, r.top, r.width(), r.height()).to_image();
    let stored = write_field(image, layout, field.id, value)?;
    if let Err(e) = save(path, image) {
        image
            .copy_from(&before, r.left, r.top)
            .expect("the copy fits where it came from");
        return Err(e);
    }
    Ok(stored)
}

/// Writes `value` into `field` of a BMP database, changing only the data
/// area's bytes in the file.
///
/// The field's frame, type mark and data area are read into a small window
/// whose top-left pixel is set to the image's background, so `write_field`
/// sees everything it reads: the mark, the data area and the background at
/// (0,0). Only the window's data area is written back.
fn write_bmp(
    path: &Path,
    info: &BmpInfo,
    background: Rgba<u8>,
    field: &Field,
    value: WriteValue,
) -> ApiResult<FieldData> {
    // Without an alpha channel the file stores every pixel as opaque; check
    // the frame color against what will be stored.
    let value = match value {
        WriteValue::Image(mut bitmap) if !info.has_alpha => {
            for p in bitmap.pixels_mut() {
                p.0[3] = 255;
            }
            WriteValue::Image(bitmap)
        }
        v => v,
    };
    let mut area = field.outer;
    if let Some(mark) = mark_area(field.outer) {
        area = Rect::new(
            area.left.min(mark.left),
            area.top.min(mark.top),
            area.right.max(mark.right),
            area.bottom.max(mark.bottom),
        );
    }
    let fail = |e: std::io::Error| ApiError::internal(format!("cannot save: {e}"));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(fail)?;
    let pixels = backend::bmp::read_rect(&mut file, info, area).map_err(fail)?;
    let mut window = RgbaImage::from_pixel(area.width() + 1, area.height() + 1, background);
    window
        .copy_from(&pixels, 1, 1)
        .expect("the window holds the area");
    let shift = |q: Rect| {
        Rect::new(
            q.left - area.left + 1,
            q.top - area.top + 1,
            q.right - area.left + 1,
            q.bottom - area.top + 1,
        )
    };
    let mut shifted = field.clone();
    shifted.outer = shift(field.outer);
    shifted.inner = shift(field.inner);
    let inner = shifted.inner;
    let mini = Layout {
        width: window.width(),
        height: window.height(),
        fields: vec![shifted],
        links: Vec::new(),
        records: Vec::new(),
    };
    let crop = |img: &RgbaImage| {
        image::imageops::crop_imm(img, inner.left, inner.top, inner.width(), inner.height())
            .to_image()
    };
    let before = crop(&window);
    let stored = write_field(&mut window, &mini, field.id, value)?;
    let after = crop(&window);
    let (x, y) = (field.inner.left, field.inner.top);
    let written =
        backend::bmp::write_rect(&mut file, info, x, y, &after).and_then(|()| file.sync_data());
    if let Err(e) = written {
        // Best effort: put the previous bytes back.
        let _ = backend::bmp::write_rect(&mut file, info, x, y, &before)
            .and_then(|()| file.sync_data());
        return Err(fail(e));
    }
    Ok(stored)
}

async fn put_value(
    State(state): State<Shared>,
    UrlPath((db, x, y, i)): UrlPath<(String, u32, u32, usize)>,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let parsed: ValueBody = serde_json::from_slice(&body)
        .map_err(|e| ApiError::bad_request("BadValue", format!("invalid JSON body: {e}")))?;
    write(state, db, (x, y, i), move |field| {
        to_write_value(field.data.type_name(), &parsed.value)
    })
    .await
}

async fn put_image(
    State(state): State<Shared>,
    UrlPath((db, x, y, i)): UrlPath<(String, u32, u32, usize)>,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let bitmap = image::load_from_memory_with_format(&body, image::ImageFormat::Png)
        .map_err(|e| ApiError::bad_request("BadImage", format!("body is not a PNG: {e}")))?
        .to_rgba8();
    write(state, db, (x, y, i), move |_| Ok(WriteValue::Image(bitmap))).await
}
