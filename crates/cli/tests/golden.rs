//! Golden tests: `cli` output for every reference image, as PNG and as BMP,
//! compared with files in `tests/golden/`. Run with `UPDATE_GOLDEN=1` to
//! rewrite the golden files from the current build.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs `cli` on `path`, with the path in the output replaced by `<path>`.
fn run(path: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_cli"))
        .arg(path)
        .output()
        .expect("cli should start");
    let mut text = format!("exit {}\n", out.status.code().unwrap_or(-1));
    text.push_str(&String::from_utf8(out.stdout).unwrap());
    text.push_str(&String::from_utf8(out.stderr).unwrap());
    text.replace(&path.display().to_string(), "<path>")
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

/// Copies of `testdata/<name>.png` as 24-bit bottom-up, 24-bit top-down and
/// 32-bit BMPs in a fresh temporary folder.
fn bmp_copies(name: &str) -> Vec<(String, PathBuf)> {
    let dir = std::env::temp_dir().join(format!("badba1-cli-golden-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let png = root().join(format!("testdata/{name}.png"));
    let image = backend::formats::read_image(&png).unwrap().1;
    [("bmp24", 24, false), ("bmp24td", 24, true), ("bmp32", 32, false)]
        .into_iter()
        .map(|(tag, bits, top_down)| {
            let path = dir.join(format!("{name}-{tag}.bmp"));
            std::fs::write(&path, backend::formats::bmp::encode_bmp(&image, bits, top_down)).unwrap();
            (format!("{name}-{tag}"), path)
        })
        .collect()
}

#[test]
fn reference_images() {
    for name in ["example1", "example2", "example3"] {
        let png = root().join(format!("testdata/{name}.png"));
        let png_out = run(&png);
        check(name, &png_out);
        for (tag, bmp) in bmp_copies(name) {
            let out = run(&bmp);
            assert_eq!(out, png_out, "{tag} differs from the PNG");
        }
    }
}

#[test]
fn errors() {
    // The OS error text depends on the system language; keep what precedes it.
    let missing = run(Path::new("no-such-file.png"));
    let cut = missing.find("<path>:").map_or(missing.len(), |i| i + "<path>:".len());
    check("missing", &missing[..cut]);
    let dir = std::env::temp_dir().join(format!("badba1-cli-golden-err-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let text = dir.join("text.png");
    std::fs::write(&text, b"just text").unwrap();
    check("not-png", &run(&text));
    let bmp = dir.join("text.bmp");
    std::fs::write(&bmp, b"just text").unwrap();
    check("not-bmp", &run(&bmp));
}
