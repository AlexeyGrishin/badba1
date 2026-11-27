//! The `ref` type: a reference to another record, traced along a line drawn
//! in the mark's color. Mark `.#.` / `###` / `.#.`. The data area has no
//! meaning, and references cannot be written.

use std::collections::{BTreeSet, HashSet, VecDeque};

use image::RgbaImage;

use super::{AreaView, FieldType, InputKind, Mark, ResolveCtx, Value, mark_area};
use crate::field_map::FieldMap;
use crate::layout::Field;

/// The `ref` field type.
pub struct Reference;

/// Mark shape of a `ref` field, rows top to bottom: `.#.`, `###`, `.#.`.
pub const REF_MARK: Mark = [
    [false, true, false],
    [true, true, true],
    [false, true, false],
];

/// How far apart, in pixels along each axis, two pixels of a reference line
/// may be and still connect, and how close a line must come to a frame to
/// touch it: gaps of up to `REF_REACH - 1` pixels are bridged.
pub const REF_REACH: u32 = 5;

impl FieldType for Reference {
    fn name(&self) -> &'static str {
        "ref"
    }

    fn mark(&self) -> Option<Mark> {
        Some(REF_MARK)
    }

    /// The data area is ignored; the target is found in
    /// [`resolve`](FieldType::resolve), once records are known.
    fn read(&self, _area: &AreaView<'_>, _color: [u8; 4]) -> Value {
        Value::Record(None)
    }

    /// Fills in the target record of every `ref` field.
    ///
    /// A field's line is every pixel of exactly its mark color, outside all
    /// fields' outer rectangles, connected to the mark within [`REF_REACH`].
    /// The target is the one record, other than the field's own, with a field
    /// whose frame is within [`REF_REACH`] of the line; otherwise the
    /// reference is null.
    fn resolve(&self, ctx: &ResolveCtx<'_>, fields: &mut [Field]) {
        let is_ref = |f: &Field| f.kind.name() == self.name();
        if !fields.iter().any(is_ref) {
            return;
        }
        let mut record_of = vec![0; fields.len() + 1];
        for r in ctx.records {
            for &id in &r.fields {
                record_of[id] = r.id;
            }
        }
        for i in 0..fields.len() {
            if !is_ref(&fields[i]) {
                continue;
            }
            let color = fields[i].mark_color();
            let line = trace(ctx.map, ctx.image, &fields[i], color);
            let own = record_of[fields[i].id];
            let targets: BTreeSet<usize> = touched_fields(ctx.map, &line)
                .into_iter()
                .map(|id| record_of[id])
                .filter(|&r| r != own)
                .collect();
            let record = if targets.len() == 1 {
                targets.into_iter().next()
            } else {
                None
            };
            fields[i].value = Value::Record(record);
        }
    }

    fn input_kind(&self) -> Option<InputKind> {
        None
    }

    fn cli_text(&self, value: &Value) -> String {
        match value {
            Value::Record(Some(id)) => format!("ref R{id}"),
            _ => "ref null".to_string(),
        }
    }
}

/// The pixels within [`REF_REACH`] of (`x`, `y`), clipped to the image.
fn window(map: &FieldMap, x: u32, y: u32) -> impl Iterator<Item = (u32, u32)> + use<> {
    let (w, h) = (map.width(), map.height());
    let x0 = x.saturating_sub(REF_REACH);
    let x1 = (x + REF_REACH).min(w - 1);
    let y0 = y.saturating_sub(REF_REACH);
    let y1 = (y + REF_REACH).min(h - 1);
    (y0..=y1).flat_map(move |ny| (x0..=x1).map(move |nx| (nx, ny)))
}

/// The line of a `ref` field: its mark pixels plus every pixel of `color`
/// outside all fields that is connected to them within [`REF_REACH`].
fn trace(map: &FieldMap, image: &RgbaImage, field: &Field, color: [u8; 4]) -> Vec<(u32, u32)> {
    let is_line = |x: u32, y: u32| !map.is_inside(x, y) && image.get_pixel(x, y).0 == color;
    // Lines are short, so a set beats a full-image buffer per field.
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    if let Some(area) = mark_area(field.outer) {
        for y in area.top..=area.bottom {
            for x in area.left..=area.right {
                if image.get_pixel(x, y).0 == color {
                    seen.insert((x, y));
                    queue.push_back((x, y));
                }
            }
        }
    }
    let mut line = Vec::new();
    while let Some((x, y)) = queue.pop_front() {
        line.push((x, y));
        for (nx, ny) in window(map, x, y) {
            if is_line(nx, ny) && seen.insert((nx, ny)) {
                queue.push_back((nx, ny));
            }
        }
    }
    line
}

/// Ids of the fields whose frame is within [`REF_REACH`] of the line.
fn touched_fields(map: &FieldMap, line: &[(u32, u32)]) -> BTreeSet<usize> {
    let mut touched = BTreeSet::new();
    for &(x, y) in line {
        for (nx, ny) in window(map, x, y) {
            if let Some(id) = map.frame_owner(nx, ny) {
                touched.insert(id);
            }
        }
    }
    touched
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::FRAME_COLOR;
    use crate::detect::tests::{blank, fill, frame};
    use crate::layout::{Layout, Rect};
    use image::Rgba;

    const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
    const YELLOW: Rgba<u8> = Rgba([255, 201, 14, 255]);
    const PURPLE: Rgba<u8> = Rgba([163, 73, 164, 255]);

    /// The referencing field; its mark is at x 27..=29, y 27..=29.
    const A: Rect = Rect::new(10, 30, 29, 49);
    /// The usual target, to the right of A.
    const G: Rect = Rect::new(80, 30, 99, 49);
    /// A field below G.
    const K: Rect = Rect::new(80, 70, 99, 89);
    /// A field below A.
    const B: Rect = Rect::new(10, 70, 29, 89);

    fn hline(img: &mut RgbaImage, y: u32, x0: u32, x1: u32, c: Rgba<u8>) {
        fill(img, Rect::new(x0, y, x1, y), c);
    }

    fn vline(img: &mut RgbaImage, x: u32, y0: u32, y1: u32, c: Rgba<u8>) {
        fill(img, Rect::new(x, y0, x, y1), c);
    }

    /// An image with field A carrying a `ref` mark in `mark`, and the other
    /// given fields (3px borders).
    fn scene(mark: Rgba<u8>, others: &[Rect]) -> RgbaImage {
        let mut img = blank(130, 100);
        for &r in std::iter::once(&A).chain(others) {
            frame(&mut img, r, (3, 3, 3, 3));
        }
        let area = mark_area(A).unwrap();
        for (dy, row) in REF_MARK.iter().enumerate() {
            for (dx, &on) in row.iter().enumerate() {
                if on {
                    img.put_pixel(area.left + dx as u32, area.top + dy as u32, mark);
                }
            }
        }
        img
    }

    /// The common path from A's mark: right along y=28, down x=60 to y=40,
    /// then right along y=40 up to and including `end_x`.
    fn path_to(img: &mut RgbaImage, end_x: u32, c: Rgba<u8>) {
        hline(img, 28, 30, 60, c);
        vline(img, 60, 28, 40, c);
        hline(img, 40, 60, end_x, c);
    }

    fn record_of(layout: &Layout, outer: Rect) -> usize {
        let id = layout.fields.iter().find(|f| f.outer == outer).unwrap().id;
        layout
            .records
            .iter()
            .find(|r| r.fields.contains(&id))
            .unwrap()
            .id
    }

    /// The target of A's reference.
    fn target(img: &RgbaImage) -> Option<usize> {
        let layout = crate::detect(img);
        let f = layout.fields.iter().find(|f| f.outer == A).unwrap();
        match &f.value {
            Value::Record(record) if f.type_name() == "ref" => *record,
            other => panic!("expected a ref, got {other:?}"),
        }
    }

    fn record_of_in(img: &RgbaImage, outer: Rect) -> usize {
        record_of(&crate::detect(img), outer)
    }

    // Line pixels

    #[test]
    fn line_in_the_mark_color() {
        let mut img = scene(YELLOW, &[G]);
        path_to(&mut img, 79, YELLOW);
        assert_eq!(target(&img), Some(record_of_in(&img, G)));
    }

    #[test]
    fn other_colors_are_not_part_of_the_line() {
        let mut img = scene(YELLOW, &[G]);
        path_to(&mut img, 79, PURPLE);
        assert_eq!(target(&img), None);
    }

    #[test]
    fn pixels_inside_fields_are_not_part_of_the_line() {
        // M sits on the path with a 1px frame; yellow pixels across its data
        // area would bridge to a second segment leading to G2 if they counted.
        let m = Rect::new(70, 35, 79, 45);
        let g2 = Rect::new(100, 30, 119, 49);
        let mut img = blank(130, 100);
        frame(&mut img, A, (3, 3, 3, 3));
        frame(&mut img, m, (1, 1, 1, 1));
        frame(&mut img, g2, (3, 3, 3, 3));
        let area = mark_area(A).unwrap();
        for (dy, row) in REF_MARK.iter().enumerate() {
            for (dx, &on) in row.iter().enumerate() {
                if on {
                    img.put_pixel(area.left + dx as u32, area.top + dy as u32, YELLOW);
                }
            }
        }
        path_to(&mut img, 69, YELLOW);
        hline(&mut img, 40, 71, 78, YELLOW); // inside M's data area
        // Segment to G2: 2px from M's data pixels (would bridge if they
        // counted), 11px from the line's end at x=69 (too far on its own).
        hline(&mut img, 40, 81, 99, YELLOW);
        assert_eq!(target(&img), Some(record_of_in(&img, m)));
    }

    // Gaps

    #[test]
    fn gap_of_4_pixels_connects() {
        let mut img = scene(YELLOW, &[G]);
        path_to(&mut img, 79, YELLOW);
        hline(&mut img, 40, 66, 69, WHITE);
        assert_eq!(target(&img), Some(record_of_in(&img, G)));
    }

    #[test]
    fn gap_of_5_pixels_breaks() {
        let mut img = scene(YELLOW, &[G]);
        path_to(&mut img, 79, YELLOW);
        hline(&mut img, 40, 66, 70, WHITE);
        assert_eq!(target(&img), None);
    }

    #[test]
    fn crossed_by_another_line() {
        let mut img = scene(PURPLE, &[G]);
        path_to(&mut img, 79, PURPLE);
        vline(&mut img, 70, 33, 47, YELLOW); // painted over the purple line
        assert_eq!(img.get_pixel(70, 40), &YELLOW);
        assert_eq!(target(&img), Some(record_of_in(&img, G)));
    }

    // Touching

    #[test]
    fn line_ends_at_the_frame() {
        let mut img = scene(YELLOW, &[G]);
        path_to(&mut img, 79, YELLOW);
        assert_eq!(target(&img), Some(record_of_in(&img, G)));
    }

    #[test]
    fn line_stops_4_pixels_short() {
        let mut img = scene(YELLOW, &[G]);
        path_to(&mut img, 75, YELLOW); // 76..=79 empty
        assert_eq!(target(&img), Some(record_of_in(&img, G)));
    }

    #[test]
    fn line_stops_5_pixels_short() {
        let mut img = scene(YELLOW, &[G]);
        path_to(&mut img, 74, YELLOW); // 75..=79 empty
        assert_eq!(target(&img), None);
    }

    // Target is a record

    /// Links G and K with a frame-color stroke, so they form one record.
    fn link_g_k(img: &mut RgbaImage) {
        vline(img, 90, 50, 69, FRAME_COLOR);
    }

    #[test]
    fn touching_any_field_of_the_record() {
        let mut img = scene(YELLOW, &[G, K]);
        link_g_k(&mut img);
        hline(&mut img, 28, 30, 60, YELLOW);
        vline(&mut img, 60, 28, 80, YELLOW);
        hline(&mut img, 80, 60, 79, YELLOW); // reaches K, G's record's last field
        let layout = crate::detect(&img);
        let rec = record_of(&layout, G);
        assert_eq!(rec, record_of(&layout, K));
        assert_eq!(target(&img), Some(rec));
    }

    #[test]
    fn touching_several_fields_of_one_record() {
        let mut img = scene(YELLOW, &[G, K]);
        link_g_k(&mut img);
        path_to(&mut img, 79, YELLOW);
        vline(&mut img, 60, 40, 80, YELLOW);
        hline(&mut img, 80, 60, 79, YELLOW);
        assert_eq!(target(&img), Some(record_of_in(&img, G)));
    }

    #[test]
    fn own_record_is_ignored() {
        let mut img = scene(YELLOW, &[G, B]);
        vline(&mut img, 20, 50, 69, FRAME_COLOR); // links A and B
        path_to(&mut img, 79, YELLOW);
        vline(&mut img, 60, 40, 80, YELLOW);
        hline(&mut img, 80, 30, 60, YELLOW); // reaches B too
        assert_eq!(record_of_in(&img, A), record_of_in(&img, B));
        assert_eq!(target(&img), Some(record_of_in(&img, G)));
    }

    // Null references

    #[test]
    fn line_reaches_nothing() {
        let mut img = scene(YELLOW, &[G]);
        path_to(&mut img, 70, YELLOW);
        assert_eq!(target(&img), None);
        // No line beyond the mark at all.
        assert_eq!(target(&scene(YELLOW, &[G])), None);
    }

    #[test]
    fn line_reaches_only_its_own_record() {
        let mut img = scene(YELLOW, &[G, B]);
        vline(&mut img, 20, 50, 69, FRAME_COLOR); // links A and B
        hline(&mut img, 28, 30, 60, YELLOW);
        vline(&mut img, 60, 28, 80, YELLOW);
        hline(&mut img, 80, 30, 60, YELLOW);
        assert_eq!(target(&img), None);
    }

    #[test]
    fn line_reaches_two_records() {
        let mut img = scene(YELLOW, &[G, K]); // not linked: two records
        path_to(&mut img, 79, YELLOW);
        vline(&mut img, 60, 40, 80, YELLOW);
        hline(&mut img, 80, 60, 79, YELLOW);
        assert_ne!(record_of_in(&img, G), record_of_in(&img, K));
        assert_eq!(target(&img), None);
    }

    // Mark, data area, value form, writes

    #[test]
    fn ref_mark() {
        let img = crate::types::testing::field_with_mark(REF_MARK, YELLOW);
        let f = crate::types::testing::field_of(&img);
        assert_eq!((f.type_name(), f.color), ("ref", Some(YELLOW.0)));
        assert_eq!(f.value, Value::Record(None));
    }

    #[test]
    fn data_area_is_ignored() {
        use crate::types::testing::{data_of, field_with_mark};
        let empty = field_with_mark(REF_MARK, YELLOW);
        let mut drawn = empty.clone();
        fill(&mut drawn, Rect::new(15, 15, 19, 19), YELLOW);
        fill(&mut drawn, Rect::new(20, 20, 24, 24), Rgba([0, 0, 0, 255]));
        assert_eq!(data_of(&drawn), data_of(&empty));
        assert_eq!(data_of(&drawn).0, "ref");
    }

    #[test]
    fn cli_text() {
        assert_eq!(Reference.cli_text(&Value::Record(Some(2))), "ref R2");
        assert_eq!(Reference.cli_text(&Value::Record(None)), "ref null");
    }

    #[test]
    fn not_writable() {
        use crate::draw::WriteError;
        use crate::types::Input;
        use crate::types::testing::{fails, scene};
        let err = fails(scene(10, 10, Some(REF_MARK)), Input::Int(1));
        assert!(matches!(
            err,
            WriteError::NotWritable {
                type_name: "ref",
                ..
            }
        ));
    }
}
