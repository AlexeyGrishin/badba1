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
        lines[0].ends_with(": 539x348, 5 fields, 3 links"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "  F1  outer (101,22)-(137,47) 37x26  border t5 r5 b5 l5  data (106,27)-(132,42) 27x16\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.ends_with("Links:\n  L1  F1 - F2\n  L2  F1 - F3\n  L3  F4 - F5\n"),
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
