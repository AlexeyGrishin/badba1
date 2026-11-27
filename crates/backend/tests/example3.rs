//! The third reference image `testdata/example3.png` (a copy of the user's
//! `db/example1.png` with a string field) must keep its pinned text.

use std::path::PathBuf;

use backend::{Rect, Value, load_db};

fn example3() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/example3.png")
}

#[test]
fn example3_string_field() {
    let db = load_db(example3()).expect("testdata/example3.png should load");
    let layout = db.layout();
    let field = layout
        .fields
        .iter()
        .find(|f| f.outer == Rect::new(297, 54, 338, 60))
        .expect("string field at (297,54)-(338,60)");
    assert_eq!(field.type_name(), "string");
    assert_eq!(field.color, Some([0, 0, 0, 255]));
    assert_eq!(field.value, Value::Text("OPENSPEC".to_string()));
}
