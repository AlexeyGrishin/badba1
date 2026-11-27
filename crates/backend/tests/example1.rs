//! The reference image `testdata/example1.png` must keep its known layout.

use std::path::PathBuf;

use backend::{Borders, Rect, Value, load_db};

fn example1() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/example1.png")
}

#[test]
fn example1_layout() {
    let layout = load_db(example1())
        .expect("testdata/example1.png should load")
        .layout()
        .clone();
    assert_eq!((layout.width, layout.height), (539, 348));

    let outers: Vec<Rect> = layout.fields.iter().map(|f| f.outer).collect();
    assert_eq!(
        outers,
        vec![
            Rect::new(101, 22, 137, 47),
            Rect::new(156, 22, 193, 50),
            Rect::new(35, 23, 77, 52),
            Rect::new(130, 69, 167, 97),
            Rect::new(72, 70, 107, 97),
        ]
    );
    let five = Borders {
        top: 5,
        right: 5,
        bottom: 5,
        left: 5,
    };
    assert!(layout.fields.iter().all(|f| f.borders == five));

    // Fields by rectangle, to state links the way the spec does.
    let id = |r: Rect| layout.fields.iter().find(|f| f.outer == r).unwrap().id;
    let smiley = id(Rect::new(35, 23, 77, 52));
    let dots = id(Rect::new(101, 22, 137, 47));
    let green = id(Rect::new(156, 22, 193, 50));
    let frown = id(Rect::new(72, 70, 107, 97));
    let red = id(Rect::new(130, 69, 167, 97));
    let sorted = |a: usize, b: usize| vec![a.min(b), a.max(b)];

    let mut links: Vec<Vec<usize>> = layout.links.iter().map(|l| l.fields.clone()).collect();
    links.sort();
    let mut expected = vec![
        sorted(smiley, dots),
        sorted(dots, green),
        sorted(frown, red),
    ];
    expected.sort();
    assert_eq!(links, expected);

    // Data types and values, read from the 3×3 marks.
    let data = |r: Rect| {
        let f = layout.fields.iter().find(|f| f.outer == r).unwrap();
        (f.type_name(), f.color, f.value.clone())
    };
    const GREEN: [u8; 4] = [34, 177, 76, 255];
    const RED: [u8; 4] = [237, 28, 36, 255];
    assert_eq!(
        data(Rect::new(101, 22, 137, 47)),
("counter", Some(GREEN), Value::Int(5))
    );
    assert_eq!(
        data(Rect::new(156, 22, 193, 50)),
("float", Some(GREEN), Value::Number(100.0 * 196.0 / 532.0))
    );
    assert_eq!(
        data(Rect::new(130, 69, 167, 97)),
("counter", Some(RED), Value::Int(3))
    );
    assert_eq!(
        data(Rect::new(35, 23, 77, 52)),
        (
            "image",
            None,
            Value::Bitmap {
                width: 33,
                height: 20
            }
        )
    );
    assert_eq!(
        data(Rect::new(72, 70, 107, 97)),
        (
            "image",
            None,
            Value::Bitmap {
                width: 26,
                height: 18
            }
        )
    );

    // Records: fields as outer rectangles, in record order.
    let records: Vec<Vec<Rect>> = layout
        .records
        .iter()
        .map(|r| {
            r.fields
                .iter()
                .map(|&id| layout.fields[id - 1].outer)
                .collect()
        })
        .collect();
    assert_eq!(
        records,
        vec![
            vec![
                Rect::new(101, 22, 137, 47),
                Rect::new(156, 22, 193, 50),
                Rect::new(35, 23, 77, 52),
            ],
            vec![Rect::new(130, 69, 167, 97), Rect::new(72, 70, 107, 97)],
        ]
    );
    assert_eq!(
        layout.records.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![1, 2]
    );
}
