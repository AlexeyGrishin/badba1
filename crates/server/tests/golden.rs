//! Golden tests: a scripted session of REST requests against each reference
//! database, as PNG and as BMP. Every request's status, content type and body
//! (JSON, or a hash of binary bodies) and the database file's hash after each
//! write are recorded and compared with `tests/golden/<db>-<format>.txt`.
//! Run with `UPDATE_GOLDEN=1` to rewrite the golden files from the current build.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use image::{Rgba, RgbaImage};
use serde_json::{Value, json};
use tower::ServiceExt;

/// FNV-1a, 64-bit: a stable hash for binary bodies and files.
fn hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x} ({} bytes)", bytes.len())
}

fn png(img: &RgbaImage) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

struct Session {
    app: Router,
    file: PathBuf,
    log: String,
}

impl Session {
    async fn send(&mut self, method: &str, uri: &str, ctype: &str, body: Vec<u8>) -> Value {
        let mut req = Request::builder().method(method).uri(uri);
        if !ctype.is_empty() {
            req = req.header("content-type", ctype);
        }
        let res = self
            .app
            .clone()
            .oneshot(req.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let rtype = res
            .headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap().to_string())
            .unwrap_or_default();
        let bytes = res.into_body().collect().await.unwrap().to_bytes().to_vec();
        let _ = writeln!(self.log, "{method} {uri}\n  {status} {rtype}");
        let value = serde_json::from_slice::<Value>(&bytes).ok();
        match &value {
            Some(v) if rtype.contains("json") => {
                let _ = writeln!(self.log, "  {v}");
            }
            _ => {
                let _ = writeln!(self.log, "  body {}", hash(&bytes));
            }
        }
        if method == "PUT" {
            let file = std::fs::read(&self.file).unwrap();
            let _ = writeln!(self.log, "  file {}", hash(&file));
        }
        value.unwrap_or(Value::Null)
    }

    async fn get(&mut self, uri: &str) -> Value {
        self.send("GET", uri, "", Vec::new()).await
    }

    async fn put_json(&mut self, uri: &str, v: Value) -> Value {
        let body = json!({ "value": v }).to_string().into_bytes();
        self.send("PUT", uri, "application/json", body).await
    }

    async fn put_png(&mut self, uri: &str, img: &RgbaImage) -> Value {
        self.send("PUT", uri, "image/png", png(img)).await
    }
}

fn temp_dir() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "badba1-server-golden-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn check(name: &str, actual: &str) {
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.txt"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&golden)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", golden.display()));
    assert_eq!(actual, expected.replace("\r\n", "\n"), "golden {name}");
}

/// Copies `testdata/<name>.png` into `dir` as `<name>.png` or as a 24-bit
/// `<name>.bmp`.
fn install(name: &str, bmp: bool, dir: &Path) -> PathBuf {
    let src =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../testdata/{name}.png"));
    if bmp {
        let image = image::open(&src).unwrap().into_rgba8();
        let dest = dir.join(format!("{name}.bmp"));
        std::fs::write(&dest, backend::formats::bmp::encode_bmp(&image, 24, false)).unwrap();
        dest
    } else {
        let dest = dir.join(format!("{name}.png"));
        std::fs::copy(src, &dest).unwrap();
        dest
    }
}

async fn session(name: &str, bmp: bool) -> String {
    let dir = temp_dir();
    let file = install(name, bmp, &dir);
    let mut s = Session {
        app: server::app(dir.clone()),
        file,
        log: String::new(),
    };
    let base = format!("/api/dbs/{name}");
    s.get("/api/dbs").await;
    let page = s.get(&format!("{base}/records?limit=1000")).await;
    s.get(&format!("{base}/records?offset=1&limit=1")).await;
    s.get(&format!("{base}/records/1/1/fields/0/image")).await;
    let records = page["records"].as_array().cloned().unwrap_or_default();
    for r in &records {
        let id = r["id"].as_str().unwrap();
        let fields = r["fields"].as_array().unwrap();
        let past = fields.len();
        s.get(&format!("{base}/records/{id}/fields/{past}/image"))
            .await;
        s.put_json(&format!("{base}/records/{id}/fields/{past}"), json!(1))
            .await;
        for (i, f) in fields.iter().enumerate() {
            let uri = format!("{base}/records/{id}/fields/{i}");
            s.get(&format!("{uri}/image")).await;
            s.put_json(&uri, json!("wrong")).await;
            match f["type"].as_str().unwrap() {
                "counter" => {
                    s.put_json(&uri, json!(2)).await;
                    s.put_json(&uri, json!(0)).await;
                    s.put_json(&uri, json!(100_000)).await;
                    s.put_json(&uri, json!(-1)).await;
                }
                "float" => {
                    s.put_json(&uri, json!(33.3)).await;
                    s.put_json(&uri, json!(100)).await;
                    s.put_json(&uri, json!(100.5)).await;
                }
                "bool" => {
                    s.put_json(&uri, json!(true)).await;
                    s.put_json(&uri, json!(false)).await;
                    s.put_json(&uri, json!(1)).await;
                }
                "string" => {
                    s.put_json(&uri, json!("AB")).await;
                    s.put_json(&uri, json!("Hi")).await;
                    s.put_json(&uri, json!("A\nB")).await;
                    s.put_json(&uri, json!("ABCDEFGHIJKLMNOPQRSTUVWXYZ")).await;
                }
                "ref" => {
                    s.put_json(&uri, json!(null)).await;
                }
                "image" => {
                    s.put_json(&uri, json!(1)).await;
                    let image_uri = format!("{uri}/image");
                    let mut small = RgbaImage::from_pixel(5, 4, Rgba([255, 0, 0, 255]));
                    small.put_pixel(1, 1, Rgba([0, 0, 255, 128]));
                    s.put_png(&image_uri, &small).await;
                    s.get(&image_uri).await;
                    let big = RgbaImage::from_pixel(500, 500, Rgba([0, 0, 0, 255]));
                    s.put_png(&image_uri, &big).await;
                    let framed = RgbaImage::from_pixel(2, 2, Rgba([0xBA, 0xDB, 0xA1, 0xFF]));
                    s.put_png(&image_uri, &framed).await;
                    s.send("PUT", &image_uri, "image/png", b"nope".to_vec())
                        .await;
                }
                other => panic!("unexpected type {other}"),
            }
        }
        if let Some(f) = fields.iter().position(|f| f["type"] != "image") {
            s.put_png(
                &format!("{base}/records/{id}/fields/{f}/image"),
                &RgbaImage::new(1, 1),
            )
            .await;
        }
    }
    s.get(&format!("{base}/records?limit=1000")).await;
    let _ = std::fs::remove_dir_all(&dir);
    s.log
}

#[tokio::test]
async fn reference_sessions() {
    for name in ["example1", "example2", "example3"] {
        for (format, bmp) in [("png", false), ("bmp", true)] {
            let log = session(name, bmp).await;
            check(&format!("{name}-{format}"), &log);
        }
    }
}

#[tokio::test]
async fn ambiguous_and_unknown_databases() {
    let dir = temp_dir();
    install("example1", false, &dir);
    install("example1", true, &dir);
    std::fs::write(dir.join("notes.txt"), b"x").unwrap();
    let mut s = Session {
        app: server::app(dir.clone()),
        file: dir.join("example1.png"),
        log: String::new(),
    };
    s.get("/api/dbs").await;
    s.get("/api/dbs/example1/records").await;
    s.get("/api/dbs/notes/records").await;
    s.get("/api/dbs/..%2Fsecret/records").await;
    let _ = std::fs::remove_dir_all(&dir);
    check("ambiguous", &s.log);
}
