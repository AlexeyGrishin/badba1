//! The reference images, stored as BMP in every supported variant, give the
//! same layout as the PNG originals.

use std::path::PathBuf;

use backend::formats::bmp::encode_bmp;

fn testdata(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata")
        .join(name)
}

#[test]
fn bmp_layouts_match_png() {
    for name in ["example1", "example2", "example3"] {
        let png = testdata(&format!("{name}.png"));
        let want = backend::load_db(&png).unwrap().layout().clone();
        let image = image::open(&png).unwrap().into_rgba8();
        for (bits, top_down) in [(24, false), (24, true), (32, false)] {
            let path = std::env::temp_dir().join(format!(
                "badba1-bmp-layout-{}-{name}-{bits}-{top_down}.BMP",
                std::process::id()
            ));
            std::fs::write(&path, encode_bmp(&image, bits, top_down)).unwrap();
            let got = backend::load_db(&path).map(|db| db.layout().clone());
            std::fs::remove_file(&path).ok();
            assert_eq!(
                got.unwrap(),
                want,
                "{name}: {bits}-bit, top-down {top_down}"
            );
        }
    }
}

#[test]
fn unsupported_and_invalid_bmps_are_errors() {
    let dir = std::env::temp_dir();
    let text = dir.join(format!("badba1-{}-text.bmp", std::process::id()));
    std::fs::write(&text, "just some text").unwrap();
    let err = backend::load_db(&text).unwrap_err();
    std::fs::remove_file(&text).ok();
    assert!(matches!(err, backend::Error::Decode { .. }), "{err}");
    assert!(err.to_string().contains("as BMP"), "{err}");

    // An 8-bit palette BMP header.
    let mut b = b"BM".to_vec();
    b.extend([0u8; 8]);
    b.extend(54u32.to_le_bytes());
    b.extend(40u32.to_le_bytes());
    b.extend(4i32.to_le_bytes());
    b.extend(4i32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(8u16.to_le_bytes());
    b.extend([0u8; 24]);
    let palette = dir.join(format!("badba1-{}-palette.bmp", std::process::id()));
    std::fs::write(&palette, b).unwrap();
    let err = backend::load_db(&palette).unwrap_err();
    std::fs::remove_file(&palette).ok();
    assert!(
        matches!(err, backend::Error::Unsupported { .. }),
        "{err}"
    );
    let msg = err.to_string();
    assert!(msg.contains("palette.bmp"), "{msg}");
    assert!(msg.contains("unsupported BMP format"), "{msg}");
}
