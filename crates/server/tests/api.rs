//! REST API tests against temporary copies of `testdata/example3.png`.

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use image::{Rgba, RgbaImage};
use serde_json::{Value, json};
use tower::ServiceExt;

/// A temporary database folder, deleted when dropped.
struct TempDb(PathBuf);

impl Deref for TempDb {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A fresh folder holding `example3.png` (a copy of the reference image).
fn db_dir() -> TempDb {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "badba1-server-test-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/example3.png");
    std::fs::copy(src, dir.join("example3.png")).unwrap();
    TempDb(dir)
}

/// One app instance that serves several requests and so shares its
/// in-memory databases between them, like a running server.
struct App(Router);

impl App {
    fn new(dir: &Path) -> Self {
        App(server::app(dir.to_path_buf()))
    }

    /// Sends a request and returns status, content type and body.
    async fn send(&self, req: Request<Body>) -> (StatusCode, String, Vec<u8>) {
        let res = self.0.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let ctype = res
            .headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap().to_string())
            .unwrap_or_default();
        let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
        (status, ctype, body)
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        let (status, _, body) = self
            .send(Request::get(uri).body(Body::empty()).unwrap())
            .await;
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    async fn get_bytes(&self, uri: &str) -> (StatusCode, Vec<u8>) {
        let (status, _, body) = self
            .send(Request::get(uri).body(Body::empty()).unwrap())
            .await;
        (status, body)
    }

    async fn put_json(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        let req = Request::put(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let (status, _, body) = self.send(req).await;
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    async fn put_png(&self, uri: &str, png: Vec<u8>) -> (StatusCode, Value) {
        let req = Request::put(uri)
            .header("content-type", "image/png")
            .body(Body::from(png))
            .unwrap();
        let (status, _, body) = self.send(req).await;
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }
}

/// Sends one request to a fresh app.
async fn send(dir: &Path, req: Request<Body>) -> (StatusCode, String, Vec<u8>) {
    App::new(dir).send(req).await
}

async fn get(dir: &Path, uri: &str) -> (StatusCode, Value) {
    App::new(dir).get(uri).await
}

async fn put_json(dir: &Path, uri: &str, body: Value) -> (StatusCode, Value) {
    App::new(dir).put_json(uri, body).await
}

/// Copies a file from `testdata/` to `dest`.
fn copy_testdata(name: &str, dest: &Path) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata")
        .join(name);
    std::fs::copy(src, dest).unwrap();
}

fn png(img: &RgbaImage) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

const R1: &str = "/api/dbs/example3/records/35/21";
const R2: &str = "/api/dbs/example3/records/250/23";

// Databases

#[tokio::test]
async fn lists_databases() {
    let dir = db_dir();
    std::fs::write(dir.join("notes.txt"), "not a db").unwrap();
    let (status, body) = get(&dir, "/api/dbs").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "dbs": ["example3"] }));
}

#[tokio::test]
async fn unknown_and_unsafe_ids_are_404() {
    let dir = db_dir();
    for uri in ["/api/dbs/missing/records", "/api/dbs/..%2Fsecret/records"] {
        let (status, body) = get(&dir, uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(body["error"], "NotFound", "{uri}");
    }
}

// Records

#[tokio::test]
async fn record_ids_in_order() {
    let dir = db_dir();
    let (status, body) = get(&dir, "/api/dbs/example3/records").await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = body["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["35/21", "250/23", "35/68"]);
    assert_eq!(body["total"], 3);
    assert_eq!(body["offset"], 0);
    assert_eq!(body["limit"], 20);
}

#[tokio::test]
async fn paging() {
    let dir = db_dir();
    let (_, body) = get(&dir, "/api/dbs/example3/records?offset=1&limit=1").await;
    assert_eq!(body["total"], 3);
    assert_eq!(body["offset"], 1);
    assert_eq!(body["limit"], 1);
    assert_eq!(body["background"], "#FFFFFF");
    assert_eq!(body["records"].as_array().unwrap().len(), 1);
    assert_eq!(body["records"][0]["id"], "250/23");

    let (_, body) = get(&dir, "/api/dbs/example3/records?offset=10").await;
    assert_eq!(body["total"], 3);
    assert_eq!(body["records"], json!([]));
}

#[tokio::test]
async fn field_json() {
    let dir = db_dir();
    let (_, body) = get(&dir, "/api/dbs/example3/records").await;
    let r1 = &body["records"][0]["fields"];
    assert_eq!(
        r1[0],
        json!({"name": "field_0", "type": "image", "width": 33, "height": 20})
    );
    assert_eq!(
        r1[1],
        json!({"name": "field_1", "type": "counter", "value": 4})
    );
    assert_eq!(r1[2]["type"], "float");
    assert!((r1[2]["value"].as_f64().unwrap() - 43.42).abs() < 0.01);
    assert_eq!(
        r1[3],
        json!({"name": "field_3", "type": "ref", "value": "250/23"})
    );

    let r2 = body["records"][1]["fields"].as_array().unwrap();
    assert_eq!(
        r2.last().unwrap(),
        &json!({"name": "field_3", "type": "string", "value": "OPENSPEC"})
    );
}

// Image data

#[tokio::test]
async fn image_endpoint() {
    let dir = db_dir();
    let uri = format!("{R1}/fields/0/image");
    let (status, ctype, body) = send(&dir, Request::get(&uri).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ctype, "image/png");
    let got = image::load_from_memory(&body).unwrap().to_rgba8();
    assert_eq!(got.dimensions(), (33, 20));
    let full = image::open(dir.join("example3.png")).unwrap().to_rgba8();
    for (x, y, p) in got.enumerate_pixels() {
        assert_eq!(p, full.get_pixel(40 + x, 26 + y));
    }

    let (status, _, _) = send(
        &dir,
        Request::get(format!("{R1}/fields/1/image"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// Writes

#[tokio::test]
async fn float_write_stores_what_fits() {
    let dir = db_dir();
    let app = App::new(&dir);
    let (status, body) = app
        .put_json(&format!("{R1}/fields/2"), json!({"value": 33.3}))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "field_2");
    assert!(
        (body["value"].as_f64().unwrap() - 33.27).abs() < 0.01,
        "{body}"
    );

    let (_, page) = app.get("/api/dbs/example3/records").await;
    assert_eq!(page["records"][0]["fields"][2]["value"], body["value"]);
}

#[tokio::test]
async fn counter_write_persists_to_file() {
    let dir = db_dir();
    let (status, body) = put_json(&dir, &format!("{R1}/fields/1"), json!({"value": 3})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["value"], 3);
    let layout = backend::analyze_png(&dir.join("example3.png")).unwrap();
    let f2 = layout
        .fields
        .iter()
        .find(|f| f.outer == backend::Rect::new(101, 22, 137, 47))
        .unwrap();
    assert!(matches!(
        f2.data,
        backend::FieldData::Counter { count: 3, .. }
    ));
}

#[tokio::test]
async fn image_upload() {
    let dir = db_dir();
    let bitmap = RgbaImage::from_fn(10, 10, |x, y| {
        Rgba([(x * 20) as u8, (y * 20) as u8, 90, 255])
    });
    let req = Request::put(format!("{R1}/fields/0/image"))
        .header("content-type", "image/png")
        .body(Body::from(png(&bitmap)))
        .unwrap();
    let (status, _, body) = send(&dir, req).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["type"], "image");

    let (_, _, data) = send(
        &dir,
        Request::get(format!("{R1}/fields/0/image"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let got = image::load_from_memory(&data).unwrap().to_rgba8();
    assert_eq!(got.dimensions(), (33, 20));
    for (x, y, p) in bitmap.enumerate_pixels() {
        assert_eq!(got.get_pixel(x, y), p);
    }
}

// Errors

#[tokio::test]
async fn overflow_is_422_and_file_unchanged() {
    let dir = db_dir();
    let before = std::fs::read(dir.join("example3.png")).unwrap();
    let (status, body) = put_json(
        &dir,
        &format!("{R2}/fields/3"),
        json!({"value": "HELLO WORLD"}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "FieldOverflow");
    assert!(!body["message"].as_str().unwrap().is_empty());
    assert_eq!(std::fs::read(dir.join("example3.png")).unwrap(), before);
}

#[tokio::test]
async fn ref_is_not_writable() {
    let dir = db_dir();
    let (status, body) = put_json(&dir, &format!("{R1}/fields/3"), json!({"value": 1})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "NotWritable");
}

#[tokio::test]
async fn unknown_record_is_404() {
    let dir = db_dir();
    let (status, body) = put_json(
        &dir,
        "/api/dbs/example3/records/1/1/fields/0",
        json!({"value": 1}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "NotFound");
}

#[tokio::test]
async fn wrong_json_type_is_400() {
    let dir = db_dir();
    let (status, body) = put_json(&dir, &format!("{R1}/fields/1"), json!({"value": "abc"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "BadValue");
}

/// A PNG with a signature, an IHDR chunk declaring `width`×`height` and an
/// empty IDAT chunk: enough to read the size, with no pixel data.
fn header_only_png(width: u32, height: u32) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    let mut ihdr = b"IHDR".to_vec();
    ihdr.extend(width.to_be_bytes());
    ihdr.extend(height.to_be_bytes());
    ihdr.extend([8, 6, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend(13u32.to_be_bytes());
    png.extend(&ihdr);
    png.extend(crc32(&ihdr).to_be_bytes());
    png.extend(0u32.to_be_bytes());
    png.extend(b"IDAT");
    png.extend(crc32(b"IDAT").to_be_bytes());
    png
}

#[tokio::test]
async fn too_large_database_is_500_image_too_large() {
    let dir = db_dir();
    std::fs::write(dir.join("huge.png"), header_only_png(20001, 20000)).unwrap();
    let (status, body) = get(&dir, "/api/dbs/huge/records").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["error"], "ImageTooLarge");
    let message = body["message"].as_str().unwrap();
    assert!(message.contains("20001x20000"), "{message}");
    assert!(message.contains("400000000"), "{message}");
}

// Databases held in memory

#[tokio::test]
async fn records_are_served_from_memory_after_the_file_changes() {
    let dir = db_dir();
    let app = App::new(&dir);
    let (status, before) = app.get("/api/dbs/example3/records").await;
    assert_eq!(status, StatusCode::OK);
    copy_testdata("example1.png", &dir.join("example3.png"));
    let (status, after) = app.get("/api/dbs/example3/records").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(after, before);
    // A fresh server does see the new file.
    let (_, fresh) = get(&dir, "/api/dbs/example3/records").await;
    assert_ne!(fresh, before);
}

#[tokio::test]
async fn image_data_is_served_from_memory_after_the_file_is_deleted() {
    let dir = db_dir();
    let app = App::new(&dir);
    let full = image::open(dir.join("example3.png")).unwrap().to_rgba8();
    let (status, _) = app.get("/api/dbs/example3/records").await;
    assert_eq!(status, StatusCode::OK);
    std::fs::remove_file(dir.join("example3.png")).unwrap();
    let (status, body) = app.get_bytes(&format!("{R1}/fields/0/image")).await;
    assert_eq!(status, StatusCode::OK);
    let got = image::load_from_memory(&body).unwrap().to_rgba8();
    assert_eq!(got.dimensions(), (33, 20));
    for (x, y, p) in got.enumerate_pixels() {
        assert_eq!(p, full.get_pixel(40 + x, 26 + y));
    }
}

#[tokio::test]
async fn failed_load_is_retried() {
    let dir = db_dir();
    let app = App::new(&dir);
    std::fs::write(dir.join("bad.png"), "not a png").unwrap();
    let (status, body) = app.get("/api/dbs/bad/records").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    copy_testdata("example3.png", &dir.join("bad.png"));
    let (status, body) = app.get("/api/dbs/bad/records").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["total"], 3);
}

#[tokio::test]
async fn new_file_is_listed_and_served() {
    let dir = db_dir();
    let app = App::new(&dir);
    let (_, body) = app.get("/api/dbs").await;
    assert_eq!(body, json!({ "dbs": ["example3"] }));
    copy_testdata("example1.png", &dir.join("new.png"));
    let (_, body) = app.get("/api/dbs").await;
    assert_eq!(body, json!({ "dbs": ["example3", "new"] }));
    let (status, body) = app.get("/api/dbs/new/records").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["total"], 2);
}

#[tokio::test]
async fn later_reads_see_a_write_from_memory() {
    let dir = db_dir();
    let app = App::new(&dir);
    let (status, body) = app
        .put_json(&format!("{R1}/fields/1"), json!({"value": 5}))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    copy_testdata("example1.png", &dir.join("example3.png"));
    let (_, page) = app.get("/api/dbs/example3/records").await;
    assert_eq!(page["records"][0]["id"], "35/21");
    assert_eq!(page["records"][0]["fields"][1]["value"], 5);
}

#[tokio::test]
async fn failed_save_leaves_memory_unchanged() {
    let dir = db_dir();
    let app = App::new(&dir);
    let file_before = std::fs::read(dir.join("example3.png")).unwrap();
    let (_, image_before) = app.get_bytes(&format!("{R1}/fields/0/image")).await;
    // A directory where the temporary file should go makes every save fail.
    std::fs::create_dir(dir.join("example3.png.tmp")).unwrap();

    let (status, body) = app
        .put_json(&format!("{R1}/fields/1"), json!({"value": 7}))
        .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert!(body["error"].is_string(), "{body}");
    let (_, page) = app.get("/api/dbs/example3/records").await;
    assert_eq!(page["records"][0]["fields"][1]["value"], 4);

    let bitmap = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 255]));
    let (status, body) = app
        .put_png(&format!("{R1}/fields/0/image"), png(&bitmap))
        .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    let (_, image_after) = app.get_bytes(&format!("{R1}/fields/0/image")).await;
    assert_eq!(image_after, image_before);

    assert_eq!(
        std::fs::read(dir.join("example3.png")).unwrap(),
        file_before
    );
}

// Web page

#[tokio::test]
async fn index_is_html() {
    let dir = db_dir();
    let (status, ctype, body) = send(&dir, Request::get("/").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(ctype.starts_with("text/html"), "{ctype}");
    assert!(String::from_utf8(body).unwrap().contains("<title>badba1"));
}
