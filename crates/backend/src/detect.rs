//! Detection of fields and links in a database image.
//!
//! The algorithm works on a mask of pixels that are exactly the frame color:
//! 1. Interiors: 4-connected regions of non-frame pixels that don't touch the
//!    image edge and fill their bounding box completely.
//! 2. Frames: each interior's outer rectangle grows one side at a time while the
//!    whole new row or column is frame color. Every side must grow at least once.
//! 3. Strokes: 8-connected frame-color pixels outside all frames.
//! 4. Links: strokes that touch the frames of two or more fields.

use std::collections::{BTreeSet, VecDeque};

use image::{Rgba, RgbaImage};

use crate::layout::{Borders, Field, FieldData, Layout, Link, Rect};
use crate::references::FieldMap;

/// The frame color, `#BADBA1`, fully opaque. Only exact matches count.
pub const FRAME_COLOR: Rgba<u8> = Rgba([0xBA, 0xDB, 0xA1, 0xFF]);

const N4: [(i64, i64); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const N8: [(i64, i64); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// Whether a pixel counts as frame color.
pub fn is_frame_color(pixel: Rgba<u8>) -> bool {
    pixel == FRAME_COLOR
}

/// Detects all fields and links in `image`.
pub fn detect(image: &RgbaImage) -> Layout {
    let (width, height) = image.dimensions();
    let grid = Grid::new(image);
    let mut fields = find_fields(&grid);
    fields.sort_by_key(|f| (f.outer.top, f.outer.left));
    for (i, field) in fields.iter_mut().enumerate() {
        field.id = i + 1;
        field.data = crate::types::read_data(image, field);
    }
    let map = FieldMap::new(width, height, &fields);
    let links = find_links(&grid, &map);
    // Free the mask before resolving references; large images need the room.
    drop(grid);
    let records = crate::records::records(&fields, &links);
    crate::references::resolve_with(image, &mut fields, &records, &map);
    Layout {
        width,
        height,
        fields,
        links,
        records,
    }
}

/// The frame-color mask of an image.
struct Grid {
    width: u32,
    height: u32,
    mask: Vec<bool>,
}

impl Grid {
    fn new(image: &RgbaImage) -> Self {
        Self {
            width: image.width(),
            height: image.height(),
            mask: image.pixels().map(|p| is_frame_color(*p)).collect(),
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        y as usize * self.width as usize + x as usize
    }

    fn is_frame(&self, x: u32, y: u32) -> bool {
        self.mask[self.index(x, y)]
    }

    /// The in-bounds neighbor of (`x`, `y`) at offset (`dx`, `dy`), if any.
    fn neighbor(&self, x: u32, y: u32, (dx, dy): (i64, i64)) -> Option<(u32, u32)> {
        let nx = i64::from(x) + dx;
        let ny = i64::from(y) + dy;
        (nx >= 0 && ny >= 0 && nx < i64::from(self.width) && ny < i64::from(self.height))
            .then_some((nx as u32, ny as u32))
    }

    /// Calls `visit` with the bounding box and pixel count of each connected
    /// region of pixels whose mask value is `value`, using the given
    /// neighborhood. Regions are not stored, so a huge background costs only
    /// its BFS frontier.
    fn regions(&self, value: bool, neighbors: &[(i64, i64)], mut visit: impl FnMut(Rect, u64)) {
        let mut seen = vec![false; self.mask.len()];
        let mut queue = VecDeque::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let start = self.index(x, y);
                if seen[start] || self.mask[start] != value {
                    continue;
                }
                seen[start] = true;
                queue.push_back((x, y));
                let mut bounds = Rect::new(x, y, x, y);
                let mut count = 0u64;
                while let Some((cx, cy)) = queue.pop_front() {
                    count += 1;
                    bounds.left = bounds.left.min(cx);
                    bounds.top = bounds.top.min(cy);
                    bounds.right = bounds.right.max(cx);
                    bounds.bottom = bounds.bottom.max(cy);
                    for &offset in neighbors {
                        if let Some((nx, ny)) = self.neighbor(cx, cy, offset) {
                            let i = self.index(nx, ny);
                            if !seen[i] && self.mask[i] == value {
                                seen[i] = true;
                                queue.push_back((nx, ny));
                            }
                        }
                    }
                }
                visit(bounds, count);
            }
        }
    }

    fn row_is_frame(&self, y: u32, left: u32, right: u32) -> bool {
        (left..=right).all(|x| self.is_frame(x, y))
    }

    fn column_is_frame(&self, x: u32, top: u32, bottom: u32) -> bool {
        (top..=bottom).all(|y| self.is_frame(x, y))
    }
}

/// Finds all fields. Ids are left at 0 for the caller to assign.
fn find_fields(grid: &Grid) -> Vec<Field> {
    let mut interiors = Vec::new();
    grid.regions(false, &N4, |bounds, count| {
        if let Some(rect) = filled_rect(grid, bounds, count) {
            interiors.push(rect);
        }
    });
    interiors
        .into_iter()
        .filter_map(|inner| frame_around(grid, inner))
        .collect()
}

/// A region's bounding box, if the region (of `count` pixels) fills it exactly
/// and doesn't touch the image edge.
fn filled_rect(grid: &Grid, rect: Rect, count: u64) -> Option<Rect> {
    let touches_edge = rect.left == 0
        || rect.top == 0
        || rect.right == grid.width - 1
        || rect.bottom == grid.height - 1;
    let filled = count == u64::from(rect.width()) * u64::from(rect.height());
    (!touches_edge && filled).then_some(rect)
}

/// Grows a frame outward from `inner`. Returns a field if every side has a
/// frame at least 1 pixel thick.
fn frame_around(grid: &Grid, inner: Rect) -> Option<Field> {
    let mut outer = inner;
    loop {
        let mut grew = false;
        if outer.top > 0 && grid.row_is_frame(outer.top - 1, outer.left, outer.right) {
            outer.top -= 1;
            grew = true;
        }
        if outer.bottom + 1 < grid.height
            && grid.row_is_frame(outer.bottom + 1, outer.left, outer.right)
        {
            outer.bottom += 1;
            grew = true;
        }
        if outer.left > 0 && grid.column_is_frame(outer.left - 1, outer.top, outer.bottom) {
            outer.left -= 1;
            grew = true;
        }
        if outer.right + 1 < grid.width
            && grid.column_is_frame(outer.right + 1, outer.top, outer.bottom)
        {
            outer.right += 1;
            grew = true;
        }
        if !grew {
            break;
        }
    }
    let borders = Borders {
        top: inner.top - outer.top,
        right: outer.right - inner.right,
        bottom: outer.bottom - inner.bottom,
        left: inner.left - outer.left,
    };
    let complete = borders.top > 0 && borders.right > 0 && borders.bottom > 0 && borders.left > 0;
    complete.then_some(Field {
        id: 0,
        outer,
        inner,
        borders,
        data: FieldData::Image,
    })
}

/// Finds links between the fields in `map`.
fn find_links(grid: &Grid, map: &FieldMap) -> Vec<Link> {
    let mut field_lists = Vec::new();
    let mut seen = vec![false; grid.mask.len()];
    let mut queue = VecDeque::new();
    for y in 0..grid.height {
        for x in 0..grid.width {
            let start = grid.index(x, y);
            if seen[start] || !grid.mask[start] || map.frame_owner(x, y).is_some() {
                continue;
            }
            seen[start] = true;
            queue.push_back((x, y));
            let mut touched = BTreeSet::new();
            while let Some((cx, cy)) = queue.pop_front() {
                for &offset in &N8 {
                    let Some((nx, ny)) = grid.neighbor(cx, cy, offset) else {
                        continue;
                    };
                    let i = grid.index(nx, ny);
                    if !grid.mask[i] {
                        continue;
                    }
                    match map.frame_owner(nx, ny) {
                        Some(id) => {
                            touched.insert(id);
                        }
                        None if !seen[i] => {
                            seen[i] = true;
                            queue.push_back((nx, ny));
                        }
                        None => {}
                    }
                }
            }
            if touched.len() >= 2 {
                field_lists.push(touched.into_iter().collect::<Vec<_>>());
            }
        }
    }

    field_lists.sort();
    field_lists
        .into_iter()
        .enumerate()
        .map(|(i, fields)| Link { id: i + 1, fields })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);

    pub fn blank(width: u32, height: u32) -> RgbaImage {
        RgbaImage::from_pixel(width, height, WHITE)
    }

    pub fn fill(img: &mut RgbaImage, r: Rect, color: Rgba<u8>) {
        for y in r.top..=r.bottom {
            for x in r.left..=r.right {
                img.put_pixel(x, y, color);
            }
        }
    }

    /// Draws a frame-color frame with the given outer rectangle and side
    /// thicknesses (top, right, bottom, left), leaving the interior as is.
    pub fn frame(img: &mut RgbaImage, outer: Rect, (t, r, b, l): (u32, u32, u32, u32)) {
        let c = FRAME_COLOR;
        fill(
            img,
            Rect::new(outer.left, outer.top, outer.right, outer.top + t - 1),
            c,
        );
        fill(
            img,
            Rect::new(outer.left, outer.bottom + 1 - b, outer.right, outer.bottom),
            c,
        );
        fill(
            img,
            Rect::new(outer.left, outer.top, outer.left + l - 1, outer.bottom),
            c,
        );
        fill(
            img,
            Rect::new(outer.right + 1 - r, outer.top, outer.right, outer.bottom),
            c,
        );
    }

    /// Stamps frame-color discs of `radius` along a path given as points.
    fn stroke(img: &mut RgbaImage, points: impl IntoIterator<Item = (f64, f64)>, radius: f64) {
        for (px, py) in points {
            let (x0, x1) = ((px - radius).floor() as i64, (px + radius).ceil() as i64);
            let (y0, y1) = ((py - radius).floor() as i64, (py + radius).ceil() as i64);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (dx, dy) = (x as f64 - px, y as f64 - py);
                    let inside =
                        x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height();
                    if inside && dx * dx + dy * dy <= radius * radius {
                        img.put_pixel(x as u32, y as u32, FRAME_COLOR);
                    }
                }
            }
        }
    }

    /// Draws a ring of frame color centered at (`cx`, `cy`).
    fn ring(img: &mut RgbaImage, (cx, cy): (f64, f64), r_in: f64, r_out: f64) {
        for (x, y, p) in img.enumerate_pixels_mut() {
            let d = ((x as f64 - cx).powi(2) + (y as f64 - cy).powi(2)).sqrt();
            if d >= r_in && d <= r_out {
                *p = FRAME_COLOR;
            }
        }
    }

    fn pairs(layout: &Layout) -> Vec<Vec<usize>> {
        layout.links.iter().map(|l| l.fields.clone()).collect()
    }

    // Frame color matching

    #[test]
    fn exact_color_matches() {
        assert!(is_frame_color(Rgba([186, 219, 161, 255])));
    }

    #[test]
    fn near_miss_color_does_not_match() {
        assert!(!is_frame_color(Rgba([186, 218, 161, 255])));
    }

    #[test]
    fn different_color_does_not_match() {
        assert!(!is_frame_color(Rgba([34, 177, 76, 255])));
    }

    #[test]
    fn partly_transparent_does_not_match() {
        assert!(!is_frame_color(Rgba([186, 219, 161, 254])));
    }

    // Field detection

    #[test]
    fn square_frame_with_equal_borders() {
        let mut img = blank(80, 60);
        frame(&mut img, Rect::new(10, 10, 49, 39), (5, 5, 5, 5));
        let layout = detect(&img);
        assert_eq!(layout.fields.len(), 1);
        let f = &layout.fields[0];
        assert_eq!(f.outer, Rect::new(10, 10, 49, 39));
        assert_eq!((f.inner.width(), f.inner.height()), (30, 20));
        assert_eq!(
            f.borders,
            Borders {
                top: 5,
                right: 5,
                bottom: 5,
                left: 5
            }
        );
    }

    #[test]
    fn unequal_borders() {
        let mut img = blank(80, 60);
        frame(&mut img, Rect::new(10, 10, 49, 39), (1, 4, 7, 2));
        let layout = detect(&img);
        assert_eq!(layout.fields.len(), 1);
        assert_eq!(
            layout.fields[0].borders,
            Borders {
                top: 1,
                right: 4,
                bottom: 7,
                left: 2
            }
        );
        assert_eq!(layout.fields[0].inner, Rect::new(12, 11, 45, 32));
    }

    #[test]
    fn data_area_holds_a_drawing() {
        let mut img = blank(80, 60);
        frame(&mut img, Rect::new(10, 10, 49, 39), (5, 5, 5, 5));
        fill(&mut img, Rect::new(20, 20, 25, 22), Rgba([0, 0, 0, 255]));
        fill(
            &mut img,
            Rect::new(30, 25, 33, 28),
            Rgba([237, 28, 36, 255]),
        );
        let layout = detect(&img);
        assert_eq!(layout.fields.len(), 1);
        assert_eq!(layout.fields[0].inner, Rect::new(15, 15, 44, 34));
    }

    #[test]
    fn thin_line_field() {
        let mut img = blank(10, 10);
        frame(&mut img, Rect::new(4, 4, 6, 6), (1, 1, 1, 1));
        let layout = detect(&img);
        assert_eq!(layout.fields.len(), 1);
        assert_eq!(layout.fields[0].inner, Rect::new(5, 5, 5, 5));
    }

    #[test]
    fn closed_curve_is_not_a_field() {
        let mut img = blank(80, 80);
        ring(&mut img, (40.0, 40.0), 17.0, 20.0);
        assert!(detect(&img).fields.is_empty());
    }

    #[test]
    fn frame_color_dot_in_interior_is_not_a_field() {
        let mut img = blank(80, 60);
        frame(&mut img, Rect::new(10, 10, 49, 39), (5, 5, 5, 5));
        img.put_pixel(30, 25, FRAME_COLOR);
        assert!(detect(&img).fields.is_empty());
    }

    #[test]
    fn numbering_follows_reading_order() {
        let mut img = blank(200, 80);
        frame(&mut img, Rect::new(100, 22, 139, 47), (5, 5, 5, 5));
        frame(&mut img, Rect::new(35, 23, 77, 52), (5, 5, 5, 5));
        let layout = detect(&img);
        let ids: Vec<_> = layout
            .fields
            .iter()
            .map(|f| (f.id, f.outer.left, f.outer.top))
            .collect();
        assert_eq!(ids, vec![(1, 100, 22), (2, 35, 23)]);
    }

    // Link detection

    #[test]
    fn straight_thin_link() {
        let mut img = blank(100, 40);
        frame(&mut img, Rect::new(10, 10, 29, 29), (3, 3, 3, 3));
        frame(&mut img, Rect::new(70, 10, 89, 29), (3, 3, 3, 3));
        fill(&mut img, Rect::new(30, 20, 69, 20), FRAME_COLOR);
        let layout = detect(&img);
        assert_eq!(layout.fields.len(), 2);
        assert_eq!(pairs(&layout), vec![vec![1, 2]]);
        assert_eq!(layout.links[0].id, 1);
    }

    #[test]
    fn thick_curved_link() {
        let mut img = blank(160, 90);
        frame(&mut img, Rect::new(10, 10, 49, 39), (5, 5, 5, 5));
        frame(&mut img, Rect::new(110, 10, 149, 39), (5, 5, 5, 5));
        // A U-shaped stroke, 10 pixels wide, from the right side of the first
        // field down and around to the left side of the second.
        let path = (0..=1000).map(|i| {
            let t = std::f64::consts::PI * f64::from(i) / 1000.0;
            (80.0 - 25.0 * t.cos(), 25.0 + 30.0 * t.sin())
        });
        stroke(&mut img, path, 5.0);
        let layout = detect(&img);
        assert_eq!(layout.fields.len(), 2);
        assert_eq!(pairs(&layout), vec![vec![1, 2]]);
    }

    #[test]
    fn branching_link() {
        let mut img = blank(100, 100);
        frame(&mut img, Rect::new(10, 10, 29, 29), (3, 3, 3, 3));
        frame(&mut img, Rect::new(70, 10, 89, 29), (3, 3, 3, 3));
        frame(&mut img, Rect::new(40, 60, 59, 79), (3, 3, 3, 3));
        fill(&mut img, Rect::new(30, 20, 69, 20), FRAME_COLOR);
        fill(&mut img, Rect::new(50, 21, 50, 59), FRAME_COLOR);
        let layout = detect(&img);
        assert_eq!(layout.fields.len(), 3);
        assert_eq!(pairs(&layout), vec![vec![1, 2, 3]]);
    }

    #[test]
    fn stroke_touching_one_field_is_not_a_link() {
        let mut img = blank(100, 60);
        frame(&mut img, Rect::new(10, 10, 29, 29), (3, 3, 3, 3));
        frame(&mut img, Rect::new(70, 10, 89, 29), (3, 3, 3, 3));
        fill(&mut img, Rect::new(30, 20, 50, 20), FRAME_COLOR);
        fill(&mut img, Rect::new(40, 45, 60, 47), FRAME_COLOR);
        let layout = detect(&img);
        assert_eq!(layout.fields.len(), 2);
        assert!(layout.links.is_empty());
    }

    #[test]
    fn line_with_a_gap_is_not_a_link() {
        let mut img = blank(100, 40);
        frame(&mut img, Rect::new(10, 10, 29, 29), (3, 3, 3, 3));
        frame(&mut img, Rect::new(70, 10, 89, 29), (3, 3, 3, 3));
        fill(&mut img, Rect::new(30, 19, 69, 21), FRAME_COLOR);
        fill(&mut img, Rect::new(50, 19, 50, 21), WHITE);
        let layout = detect(&img);
        assert_eq!(layout.fields.len(), 2);
        assert!(layout.links.is_empty());
    }
}
