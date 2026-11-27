//! REST API handlers.
//!
//! The server is a client of the `backend` library: each database is a
//! [`backend::Database`], loaded with [`backend::load_db`] when a request
//! first names it and then served from memory for the rest of the run (how
//! its pixels are kept and saved is up to its storage format). Changes made
//! to the file by other programs are not seen until the server restarts. Each
//! database has its own read/write lock: reads run in parallel, writes go
//! through [`Database::set_field`], which leaves the database unchanged when
//! the save fails.
//!
//! JSON values are converted by value kind ([`backend::Value`],
//! [`backend::InputKind`]), so the server has no code for specific field
//! types or storage formats.

use std::collections::HashMap;
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
use backend::{Database, FieldView, Input, InputKind, RecordId, WriteError, formats};
use image::RgbaImage;
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

type Slot = Arc<RwLock<Option<Database>>>;

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

/// Runs `f` on database `id`, loading it first if this is its first use.
/// Concurrent first uses load it once; a failed load is not remembered.
fn with_db<T>(
    state: &AppState,
    id: &str,
    f: impl FnOnce(&Database) -> ApiResult<T>,
) -> ApiResult<T> {
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
    f: impl FnOnce(&mut Database) -> ApiResult<T>,
) -> ApiResult<T> {
    let slot = state.slot(id)?;
    let mut guard = slot.write().unwrap_or_else(PoisonError::into_inner);
    if guard.is_none() {
        let path = db_path(&state.db_dir, id)?;
        *guard = Some(backend::load_db(path)?);
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

impl From<backend::Error> for ApiError {
    fn from(e: backend::Error) -> Self {
        match e {
            backend::Error::Write(w) => w.into(),
            backend::Error::NoSuchRecord(_) | backend::Error::NoSuchField { .. } => {
                ApiError::not_found(e.to_string())
            }
            backend::Error::TooLarge { .. } => ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                kind: "ImageTooLarge",
                message: e.to_string(),
            },
            _ => ApiError::internal(e.to_string()),
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

/// The file of database `id`: `<id>.<ext>` for the extension of a
/// registered storage format. 404 if the id is invalid or no such file
/// exists, 409 if files exist in two or more formats.
fn db_path(dir: &Path, id: &str) -> ApiResult<PathBuf> {
    let not_found = || ApiError::not_found(format!("no database {id:?}"));
    if !valid_id(id) {
        return Err(not_found());
    }
    let files: Vec<String> = formats::extensions()
        .map(|ext| format!("{id}.{ext}"))
        .filter(|name| dir.join(name).is_file())
        .collect();
    match files.as_slice() {
        [] => Err(not_found()),
        [one] => Ok(dir.join(one)),
        [a, b] => Err(ApiError::conflict(
            "AmbiguousDatabase",
            format!("database {id:?} exists as both {a} and {b}"),
        )),
        [rest @ .., last] => Err(ApiError::conflict(
            "AmbiguousDatabase",
            format!("database {id:?} exists as {} and {last}", rest.join(", ")),
        )),
    }
}

fn png_bytes(image: &RgbaImage) -> ApiResult<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    image
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(out.into_inner())
}

/// The JSON of field `f` with the given name: its type and, by value kind,
/// its value (or, for a bitmap, its size).
fn field_json(name: String, f: &FieldView<'_>) -> Value {
    let mut v = json!({ "name": name, "type": f.type_name() });
    let value = match f.value() {
        backend::Value::Bitmap { width, height } => {
            v["width"] = json!(width);
            v["height"] = json!(height);
            return v;
        }
        backend::Value::Int(n) => json!(n),
        backend::Value::Number(n) => json!(n),
        backend::Value::Bool(b) => json!(b),
        backend::Value::Text(text) => json!(text),
        backend::Value::Record(_) => json!(f.target().map(|id| id.to_string())),
    };
    v["value"] = value;
    v
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
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .is_some_and(|x| formats::extensions().any(|ext| x == ext))
            })
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
fn records_page(db: &str, d: &Database, page: &Page) -> ApiResult<Json<Value>> {
    let offset = page.offset.unwrap_or(0);
    let limit = page.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let records: Vec<Value> = d
        .records()
        .skip(offset)
        .take(limit)
        .map(|r| {
            let fields: Vec<Value> = r
                .fields()
                .map(|f| field_json(format!("field_{}", f.index()), &f))
                .collect();
            json!({ "id": r.id().to_string(), "fields": fields })
        })
        .collect();
    let bg = d.background().0;
    Ok(Json(json!({
        "db": db,
        "total": d.records().len(),
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
            let record = RecordId { x, y };
            let f = d.field(record, i)?;
            if !matches!(f.value(), backend::Value::Bitmap { .. }) {
                return Err(ApiError::not_found(format!(
                    "field {i} of record {x}/{y} is not an image field"
                )));
            }
            let bytes = png_bytes(&d.field_image(record, i)?)?;
            Ok(([(header::CONTENT_TYPE, "image/png")], bytes).into_response())
        })
    })
    .await
}

#[derive(Deserialize)]
struct ValueBody {
    value: Value,
}

/// Converts a JSON value to an input of `kind` for a field of type
/// `type_name`.
fn to_input(kind: InputKind, type_name: &str, v: &Value) -> ApiResult<Input> {
    let bad = || {
        ApiError::bad_request(
            "BadValue",
            format!("{v} is not a valid value for a {type_name} field"),
        )
    };
    Ok(match kind {
        InputKind::Int => Input::Int(v.as_u64().ok_or_else(bad)?),
        InputKind::Number => Input::Number(v.as_f64().ok_or_else(bad)?),
        InputKind::Bool => Input::Bool(v.as_bool().ok_or_else(bad)?),
        InputKind::Text => Input::Text(v.as_str().ok_or_else(bad)?.to_string()),
        InputKind::Bitmap => {
            return Err(ApiError::bad_request(
                "BadValue",
                format!("{type_name} fields are not written with a JSON value"),
            ));
        }
    })
}

/// Writes the input built by `make` into field `i` of record `x/y` of
/// database `db` and returns the field's JSON. A field whose type cannot be
/// written is refused before `make` runs.
async fn write(
    state: Shared,
    db: String,
    (x, y, i): (u32, u32, usize),
    make: impl FnOnce(InputKind, &'static str) -> ApiResult<Input> + Send + 'static,
) -> ApiResult<Json<Value>> {
    blocking(move || {
        with_db_mut(&state, &db, |d| {
            let record = RecordId { x, y };
            let field = d.field(record, i)?.field();
            let type_name = field.type_name();
            let Some(kind) = field.kind.input_kind() else {
                return Err(WriteError::NotWritable {
                    field: field.id,
                    type_name,
                }
                .into());
            };
            let input = make(kind, type_name)?;
            let f = d.set_field(record, i, input)?;
            Ok(Json(field_json(format!("field_{i}"), &f)))
        })
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
    write(state, db, (x, y, i), move |kind, type_name| {
        to_input(kind, type_name, &parsed.value)
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
    write(state, db, (x, y, i), move |_, _| Ok(Input::Bitmap(bitmap))).await
}
