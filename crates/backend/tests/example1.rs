//! The reference image `testdata/example1.png` must keep its known layout.

use std::path::PathBuf;

use backend::{Borders, Rect, analyze_png};

fn example1() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/example1.png")
}

#[test]
fn example1_layout() {
    let layout = analyze_png(&example1()).expect("testdata/example1.png should load");
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
}
