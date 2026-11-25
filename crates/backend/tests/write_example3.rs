//! Writes into the reference image `testdata/example3.png` (in memory): each
//! write returns the stored value, and re-analysis shows the same layout with
//! only the written field's value changed. Failed writes change nothing.

use std::path::PathBuf;

use backend::{FieldData, Layout, Rect, WriteError, WriteValue, detect, write_field};
use image::{Rgba, RgbaImage};

fn example3() -> RgbaImage {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/example3.png");
    image::open(path)
        .expect("testdata/example3.png should load")
        .to_rgba8()
}

/// Field id with its outer and inner rectangles.
type FieldRects = (usize, Rect, Rect);
/// A layout's geometry: field rectangles, link field lists, record field lists.
type Geometry = (Vec<FieldRects>, Vec<Vec<usize>>, Vec<Vec<usize>>);

/// A layout's geometry, without field values.
fn shape(layout: &Layout) -> Geometry {
    (
        layout
            .fields
            .iter()
            .map(|f| (f.id, f.outer, f.inner))
            .collect(),
        layout.links.iter().map(|l| l.fields.clone()).collect(),
        layout.records.iter().map(|r| r.fields.clone()).collect(),
    )
}

/// Writes `value` into field `id`, checks the layout keeps its shape and only
/// that field's value changes, and returns the written field's new value.
fn write_and_check(img: &mut RgbaImage, id: usize, value: WriteValue) -> FieldData {
    let before = detect(img);
    let returned = write_field(img, &before, id, value).expect("write succeeds");
    let after = detect(img);
    assert_eq!(
        shape(&after),
        shape(&before),
        "layout changed by writing F{id}"
    );
    for (b, a) in before.fields.iter().zip(&after.fields) {
        if a.id == id {
            assert_eq!(a.data, returned, "F{id} reads differently than returned");
        } else {
            assert_eq!(a.data, b.data, "F{} changed by writing F{id}", a.id);
        }
    }
    returned
}

#[test]
fn writes_round_trip() {
    let mut img = example3();

    let got = write_and_check(&mut img, 2, WriteValue::Counter(3));
    assert!(matches!(got, FieldData::Counter { count: 3, .. }));

    let got = write_and_check(&mut img, 3, WriteValue::Float(33.3));
    assert!(matches!(
        got,
        FieldData::Float {
            filled: 177,
            total: 532,
            ..
        }
    ));

    let got = write_and_check(&mut img, 6, WriteValue::Bool(false));
    assert!(matches!(got, FieldData::Bool { value: false, .. }));

    let got = write_and_check(&mut img, 8, WriteValue::String("HELLO".into()));
    assert!(matches!(got, FieldData::String { ref text, .. } if text == "HELLO"));

    let bitmap = RgbaImage::from_fn(10, 10, |x, y| {
        Rgba([(x * 20) as u8, (y * 20) as u8, 90, 255])
    });
    let got = write_and_check(&mut img, 1, WriteValue::Image(bitmap));
    assert_eq!(got, FieldData::Image);
}

#[test]
fn failed_writes_change_nothing() {
    let mut img = example3();
    let layout = detect(&img);
    let before = img.clone();
    let cases = [
        (2, WriteValue::Counter(113)),
        (8, WriteValue::String("HELLO WORLD".into())),
        (5, WriteValue::Counter(1)),
        (2, WriteValue::String("A".into())),
        (99, WriteValue::Bool(true)),
    ];
    for (id, value) in cases {
        let err = write_field(&mut img, &layout, id, value).unwrap_err();
        assert_eq!(img, before, "image changed by failed write to F{id}: {err}");
        match id {
            2 | 8 if matches!(err, WriteError::FieldOverflow { .. }) => {}
            5 => assert!(matches!(err, WriteError::NotWritable { .. })),
            2 => assert!(matches!(err, WriteError::TypeMismatch { .. })),
            99 => assert_eq!(err, WriteError::NoSuchField(99)),
            _ => panic!("unexpected error for F{id}: {err:?}"),
        }
    }
}
