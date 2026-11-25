//! The second reference image `testdata/example2.png` (a copy of the user's
//! `db/example1.png` with bool fields) must keep its pinned bool values.

use std::path::PathBuf;

use backend::{FieldData, Rect, analyze_png};

fn example2() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/example2.png")
}

#[test]
fn example2_bool_fields() {
    let layout = analyze_png(&example2()).expect("testdata/example2.png should load");
    assert_eq!((layout.width, layout.height), (539, 348));

    let data = |r: Rect| {
        layout
            .fields
            .iter()
            .find(|f| f.outer == r)
            .unwrap_or_else(|| panic!("no field at {r:?}"))
            .data
    };
    const PINK: [u8; 4] = [255, 174, 201, 255];
    assert_eq!(
        data(Rect::new(310, 29, 327, 49)),
        FieldData::Bool {
            color: PINK,
            value: true
        }
    );
    assert_eq!(
        data(Rect::new(288, 31, 305, 49)),
        FieldData::Bool {
            color: PINK,
            value: false
        }
    );
}

#[test]
fn example2_references() {
    let layout = analyze_png(&example2()).expect("testdata/example2.png should load");
    let field_id = |r: Rect| layout.fields.iter().find(|f| f.outer == r).unwrap().id;
    let target_field = field_id(Rect::new(250, 23, 280, 52));
    let target_record = layout
        .records
        .iter()
        .find(|r| r.fields.contains(&target_field))
        .unwrap()
        .id;

    let data = |r: Rect| layout.fields[field_id(r) - 1].data;
    // Yellow `+`, line ending at the target's bottom frame.
    assert_eq!(
        data(Rect::new(198, 26, 220, 43)),
        FieldData::Ref {
            color: [255, 201, 14, 255],
            record: Some(target_record)
        }
    );
    // Purple `+`, line crossed by the yellow one (1px gap), ending at the
    // target's left frame.
    assert_eq!(
        data(Rect::new(213, 74, 234, 93)),
        FieldData::Ref {
            color: [163, 73, 164, 255],
            record: Some(target_record)
        }
    );
}
