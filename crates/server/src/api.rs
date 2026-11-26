//! REST API handlers.
//!
//! Every request re-reads the database file and re-runs detection. Writes hold
//! a lock for their whole load → write → save sequence and save through a
//! temporary file and a rename, so a database is never half-written.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use backend::{Field, FieldData, Layout, WriteError, WriteValue, detect, write_field};
use image::RgbaImage;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::Mutex;

/// The web page, served at `/`.
const INDEX_HTML: &str = include_str!("../ui/index.html");

/// Default and maximum page sizes for the records endpoint.
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 1000;

struct AppState {
    db_dir: PathBuf,
    write_lock: Mutex<()>,
}

type Shared = Arc<AppState>;

/// The server's routes, serving the databases in `db_dir`.
pub fn app(db_dir: PathBuf) -> Router {
    let state = Arc::new(AppState {
        db_dir,
        write_lock: Mutex::new(()),
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

/// The file of database `id`, if the id is valid and the file exists.
fn db_path(dir: &Path, id: &str) -> ApiResult<PathBuf> {
    let path = dir.join(format!("{id}.png"));
    if valid_id(id) && path.is_file() {
        Ok(path)
    } else {
        Err(ApiError::not_found(format!("no database {id:?}")))
    }
}

fn load(path: &Path) -> ApiResult<RgbaImage> {
    backend::load_png(path).map_err(|e| match e {
        backend::Error::TooLarge { .. } => ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            kind: "ImageTooLarge",
            message: e.to_string(),
        },
        _ => ApiError::internal(e.to_string()),
    })
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
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "png"))
            .filter_map(|p| p.file_stem()?.to_str().map(str::to_string))
            .filter(|id| valid_id(id))
            .collect();
        dbs.sort();
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
    let dir = state.db_dir.clone();
    blocking(move || {
        let path = db_path(&dir, &db)?;
        let image = load(&path)?;
        let layout = detect(&image);
        let ids = record_ids(&layout);
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
                        field_json(format!("field_{i}"), f, &f.data, &ids)
                    })
                    .collect();
                json!({ "id": ids[r.id], "fields": fields })
            })
            .collect();
        let bg = image.get_pixel(0, 0).0;
        Ok(Json(json!({
            "db": db,
            "total": layout.records.len(),
            "offset": offset,
            "limit": limit,
            "background": format!("#{:02X}{:02X}{:02X}", bg[0], bg[1], bg[2]),
            "records": records,
        })))
    })
    .await
}

async fn get_image(
    State(state): State<Shared>,
    UrlPath((db, x, y, i)): UrlPath<(String, u32, u32, usize)>,
) -> ApiResult<Response> {
    let dir = state.db_dir.clone();
    blocking(move || {
        let image = load(&db_path(&dir, &db)?)?;
        let layout = detect(&image);
        let f = find_field(&layout, x, y, i)?;
        if f.data != FieldData::Image {
            return Err(ApiError::not_found(format!(
                "field {i} of record {x}/{y} is not an image field"
            )));
        }
        let r = f.inner;
        let crop =
            image::imageops::crop_imm(&image, r.left, r.top, r.width(), r.height()).to_image();
        let bytes = png_bytes(&crop)?;
        Ok(([(header::CONTENT_TYPE, "image/png")], bytes).into_response())
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

/// Loads database `db`, writes the value built by `make` into field `i` of
/// record `x/y`, saves, and returns the field's JSON.
async fn write(
    state: Shared,
    db: String,
    (x, y, i): (u32, u32, usize),
    make: impl FnOnce(&Field) -> ApiResult<WriteValue> + Send + 'static,
) -> ApiResult<Json<Value>> {
    let _guard = state.write_lock.lock().await;
    let dir = state.db_dir.clone();
    blocking(move || {
        let path = db_path(&dir, &db)?;
        let mut image = load(&path)?;
        let layout = detect(&image);
        let field = find_field(&layout, x, y, i)?.clone();
        if let FieldData::Ref { .. } = field.data {
            return Err(WriteError::NotWritable {
                field: field.id,
                type_name: "ref",
            }
            .into());
        }
        let value = make(&field)?;
        let stored = write_field(&mut image, &layout, field.id, value)?;
        save(&path, &image)?;
        let ids = record_ids(&layout);
        Ok(Json(field_json(
            format!("field_{i}"),
            &field,
            &stored,
            &ids,
        )))
    })
    .await
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
