//! References: `ref` fields point to another record along a line drawn in the
//! mark's color.

use std::collections::{BTreeSet, VecDeque};

use image::RgbaImage;

use crate::layout::{Field, FieldData, Record};
use crate::types::mark_area;

/// How far apart (horizontally and vertically) two line pixels, or a line
/// pixel and a frame pixel, may be and still count as connected: 5, which
/// allows gaps of up to 4 pixels.
pub const REF_REACH: u32 = 5;

/// Fills in the target record of every `ref` field in `fields`.
///
/// A field's line is every pixel of exactly its mark color, outside all
/// fields' outer rectangles, connected to the mark within [`REF_REACH`]. The
/// target is the one record, other than the field's own, with a field whose
/// frame is within [`REF_REACH`] of the line; otherwise the reference is null.
pub fn resolve(image: &RgbaImage, fields: &mut [Field], records: &[Record]) {
    if !fields
        .iter()
        .any(|f| matches!(f.data, FieldData::Ref { .. }))
    {
        return;
    }
    let grid = Grid::new(image, fields);
    let mut record_of = vec![0; fields.len() + 1];
    for r in records {
        for &id in &r.fields {
            record_of[id] = r.id;
        }
    }
    for i in 0..fields.len() {
        let FieldData::Ref { color, .. } = fields[i].data else {
            continue;
        };
        let line = grid.trace(image, &fields[i], color);
        let own = record_of[fields[i].id];
        let targets: BTreeSet<usize> = grid
            .touched_fields(&line)
            .into_iter()
            .map(|id| record_of[id])
            .filter(|&r| r != own)
            .collect();
        let record = if targets.len() == 1 {
            targets.into_iter().next()
        } else {
            None
        };
        fields[i].data = FieldData::Ref { color, record };
    }
}

/// Per-pixel facts about fields, shared by all references in an image.
struct Grid {
    width: u32,
    height: u32,
    /// Whether the pixel is inside some field's outer rectangle.
    inside: Vec<bool>,
    /// The id of the field whose frame the pixel belongs to, if any.
    frame_owner: Vec<Option<usize>>,
}

impl Grid {
    fn new(image: &RgbaImage, fields: &[Field]) -> Self {
        let (width, height) = image.dimensions();
        let n = width as usize * height as usize;
        let mut grid = Grid {
            width,
            height,
            inside: vec![false; n],
            frame_owner: vec![None; n],
        };
        for f in fields {
            for y in f.outer.top..=f.outer.bottom {
                for x in f.outer.left..=f.outer.right {
                    let i = grid.index(x, y);
                    grid.inside[i] = true;
                    if !f.inner.contains(x, y) {
                        grid.frame_owner[i] = Some(f.id);
                    }
                }
            }
        }
        grid
    }

    fn index(&self, x: u32, y: u32) -> usize {
        y as usize * self.width as usize + x as usize
    }

    /// The pixels within [`REF_REACH`] of (`x`, `y`), clipped to the image.
    fn window(&self, x: u32, y: u32) -> impl Iterator<Item = (u32, u32)> + use<> {
        let (w, h) = (self.width, self.height);
        let x0 = x.saturating_sub(REF_REACH);
        let x1 = (x + REF_REACH).min(w - 1);
        let y0 = y.saturating_sub(REF_REACH);
        let y1 = (y + REF_REACH).min(h - 1);
        (y0..=y1).flat_map(move |ny| (x0..=x1).map(move |nx| (nx, ny)))
    }

    /// The line of a `ref` field: its mark pixels plus every pixel of `color`
    /// outside all fields that is connected to them within [`REF_REACH`].
    fn trace(&self, image: &RgbaImage, field: &Field, color: [u8; 4]) -> Vec<(u32, u32)> {
        let is_line =
            |x: u32, y: u32| !self.inside[self.index(x, y)] && image.get_pixel(x, y).0 == color;
        let mut seen = vec![false; self.inside.len()];
        let mut queue = VecDeque::new();
        if let Some(area) = mark_area(field.outer) {
            for y in area.top..=area.bottom {
                for x in area.left..=area.right {
                    if image.get_pixel(x, y).0 == color {
                        seen[self.index(x, y)] = true;
                        queue.push_back((x, y));
                    }
                }
            }
        }
        let mut line = Vec::new();
        while let Some((x, y)) = queue.pop_front() {
            line.push((x, y));
            for (nx, ny) in self.window(x, y) {
                let i = self.index(nx, ny);
                if !seen[i] && is_line(nx, ny) {
                    seen[i] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
        line
    }

    /// Ids of the fields whose frame is within [`REF_REACH`] of the line.
    fn touched_fields(&self, line: &[(u32, u32)]) -> BTreeSet<usize> {
        let mut touched = BTreeSet::new();
        for &(x, y) in line {
            for (nx, ny) in self.window(x, y) {
                if let Some(id) = self.frame_owner[self.index(nx, ny)] {
                    touched.insert(id);
                }
            }
        }
        touched
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::FRAME_COLOR;
    use crate::detect::tests::{blank, fill, frame};
    use crate::layout::{Layout, Rect};
    use crate::types::REF_MARK;
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
        match &layout.fields.iter().find(|f| f.outer == A).unwrap().data {
            FieldData::Ref { record, .. } => *record,
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
}
