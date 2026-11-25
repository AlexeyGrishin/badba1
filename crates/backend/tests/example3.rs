//! The third reference image `testdata/example3.png` (a copy of the user's
//! `db/example1.png` with a string field) must keep its pinned text.

use std::path::PathBuf;

use backend::{FieldData, Rect, analyze_png};

fn example3() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/example3.png")
}

#[test]
fn example3_string_field() {
    let layout = analyze_png(&example3()).expect("testdata/example3.png should load");
    let field = layout
        .fields
        .iter()
        .find(|f| f.outer == Rect::new(297, 54, 338, 60))
        .expect("string field at (297,54)-(338,60)");
    assert_eq!(
        field.data,
        FieldData::String {
            color: [0, 0, 0, 255],
            text: "OPENSPEC".to_string()
        }
    );
}
