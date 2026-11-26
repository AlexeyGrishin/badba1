//! Runs the built `cli` binary and checks its output and exit codes.

use std::path::PathBuf;
use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cli"))
        .args(args)
        .output()
        .expect("cli should start")
}

fn example1() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/example1.png")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn reference_image() {
    let out = cli(&[&example1()]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines[0].ends_with(": 539x348, 5 fields, 3 links, 2 records"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "  F1  outer (101,22)-(137,47) 37x26  border t5 r5 b5 l5  data (106,27)-(132,42) 27x16  counter 5\n"
        ),
        "{stdout}"
    );
    let field_line = |id: &str| {
        lines
            .iter()
            .find(|l| l.starts_with(&format!("  {id}  ")))
            .copied()
            .unwrap_or_else(|| panic!("no {id} line in {stdout}"))
    };
    assert!(field_line("F2").ends_with("  float 36.84%"), "{stdout}");
    assert!(field_line("F3").ends_with(" 33x20  image"), "{stdout}");
    assert!(field_line("F4").ends_with("  counter 3"), "{stdout}");
    assert!(field_line("F5").ends_with("  image"), "{stdout}");
    assert!(
        stdout.ends_with(
            "Links:\n  L1  F1 - F2\n  L2  F1 - F3\n  L3  F4 - F5\nRecords:\n  R1  F1, F2, F3\n  R2  F4, F5\n"
        ),
        "{stdout}"
    );
}

#[test]
fn no_arguments_prints_usage() {
    let out = cli(&[]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("usage: cli <image.png>"), "{stderr}");
    assert!(out.stdout.is_empty());
}

#[test]
fn too_many_arguments_prints_usage() {
    let out = cli(&["a.png", "b.png"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn missing_file() {
    let out = cli(&["does-not-exist.png"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("does-not-exist.png"), "{stderr}");
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

#[test]
fn too_large_image_is_a_file_error() {
    let path = std::env::temp_dir().join(format!("badba1-cli-{}-huge.png", std::process::id()));
    std::fs::write(&path, header_only_png(20001, 20000)).unwrap();
    let out = cli(&[&path.to_string_lossy()]);
    std::fs::remove_file(&path).ok();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("huge.png"), "{stderr}");
    assert!(stderr.contains("too large"), "{stderr}");
    assert!(stderr.contains("400000000"), "{stderr}");
}
