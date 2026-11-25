//! Field data types, read from a 3×3 mark above each field's top-right corner,
//! and the values of typed fields.

use std::collections::VecDeque;

use image::RgbaImage;

use crate::detect::FRAME_COLOR;
use crate::layout::{Field, FieldData, Rect};

/// Mark shape of a `float` field, rows top to bottom: `###`, `##.`, `#..`.
pub const FLOAT_MARK: [[bool; 3]; 3] = [
    [true, true, true],
    [true, true, false],
    [true, false, false],
];

/// Mark shape of a `counter` field, rows top to bottom: `###`, `#..`, `###`.
pub const COUNTER_MARK: [[bool; 3]; 3] =
    [[true, true, true], [true, false, false], [true, true, true]];

/// Mark shape of a `bool` field, rows top to bottom: `.#.`, `.##`, `.##`.
pub const BOOL_MARK: [[bool; 3]; 3] = [
    [false, true, false],
    [false, true, true],
    [false, true, true],
];

/// Mark shape of a `ref` field, rows top to bottom: `.#.`, `###`, `.#.`.
pub const REF_MARK: [[bool; 3]; 3] = [
    [false, true, false],
    [true, true, true],
    [false, true, false],
];

/// Mark shape of a `string` field, rows top to bottom: `.#.`, `#.#`, `#.#`.
pub const STRING_MARK: [[bool; 3]; 3] = [
    [false, true, false],
    [true, false, true],
    [true, false, true],
];

/// The 3×3 area holding a field's type mark: directly above the top-right
/// corner of the outer rectangle. `None` if there is no room above the field.
pub fn mark_area(outer: Rect) -> Option<Rect> {
    (outer.top >= 3 && outer.right >= 2)
        .then(|| Rect::new(outer.right - 2, outer.top - 3, outer.right, outer.top - 1))
}

/// Reads a field's data type from its mark and computes its value.
pub fn read_data(image: &RgbaImage, field: &Field) -> FieldData {
    let Some(area) = mark_area(field.outer) else {
        return FieldData::Image;
    };
    // The top-middle pixel belongs to every mark shape; the top-left does not
    // (the `bool` mark leaves it empty).
    let color = image.get_pixel(area.left + 1, area.top).0;
    if color == FRAME_COLOR.0 {
        return FieldData::Image;
    }
    let mut shape = [[false; 3]; 3];
    for (dy, row) in shape.iter_mut().enumerate() {
        for (dx, cell) in row.iter_mut().enumerate() {
            *cell = image
                .get_pixel(area.left + dx as u32, area.top + dy as u32)
                .0
                == color;
        }
    }
    if shape == FLOAT_MARK {
        let inner = field.inner;
        let filled = pixels(inner)
            .filter(|&(x, y)| image.get_pixel(x, y).0 == color)
            .count() as u64;
        let total = u64::from(inner.width()) * u64::from(inner.height());
        FieldData::Float {
            color,
            filled,
            total,
        }
    } else if shape == COUNTER_MARK {
        FieldData::Counter {
            color,
            count: count_objects(image, field.inner, color),
        }
    } else if shape == BOOL_MARK {
        FieldData::Bool {
            color,
            value: pixels(field.inner).any(|(x, y)| image.get_pixel(x, y).0 == color),
        }
    } else if shape == REF_MARK {
        // The data area is ignored; the target is resolved later, once records
        // are known (see `references::resolve`).
        FieldData::Ref {
            color,
            record: None,
        }
    } else if shape == STRING_MARK {
        FieldData::String {
            color,
            text: crate::strings::read_text(image, field.inner, crate::font::builtin()),
        }
    } else {
        FieldData::Image
    }
}

fn pixels(r: Rect) -> impl Iterator<Item = (u32, u32)> {
    (r.top..=r.bottom).flat_map(move |y| (r.left..=r.right).map(move |x| (x, y)))
}

/// Counts groups of `color` pixels inside `area` that are connected
/// horizontally or vertically.
fn count_objects(image: &RgbaImage, area: Rect, color: [u8; 4]) -> u64 {
    let (w, h) = (area.width() as usize, area.height() as usize);
    let is_color =
        |x: usize, y: usize| image.get_pixel(area.left + x as u32, area.top + y as u32).0 == color;
    let mut seen = vec![false; w * h];
    let mut queue = VecDeque::new();
    let mut count = 0;
    for y in 0..h {
        for x in 0..w {
            if seen[y * w + x] || !is_color(x, y) {
                continue;
            }
            count += 1;
            seen[y * w + x] = true;
            queue.push_back((x, y));
            while let Some((cx, cy)) = queue.pop_front() {
                let neighbors = [
                    (cx.wrapping_sub(1), cy),
                    (cx + 1, cy),
                    (cx, cy.wrapping_sub(1)),
                    (cx, cy + 1),
                ];
                for (nx, ny) in neighbors {
                    if nx < w && ny < h && !seen[ny * w + nx] && is_color(nx, ny) {
                        seen[ny * w + nx] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::detect;
    use crate::detect::tests::{blank, fill, frame};
    use image::Rgba;

    const GREEN: Rgba<u8> = Rgba([34, 177, 76, 255]);
    const RED: Rgba<u8> = Rgba([237, 28, 36, 255]);
    const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);
    const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
    const PINK: Rgba<u8> = Rgba([255, 174, 201, 255]);
    const YELLOW: Rgba<u8> = Rgba([255, 201, 14, 255]);

    /// Outer rectangle of the test field: data area (15,15)-(24,24), 10×10.
    const OUTER: Rect = Rect::new(10, 10, 29, 29);

    /// An image with one field at `OUTER` (5px borders) and the given mark.
    fn field_with_mark(shape: [[bool; 3]; 3], color: Rgba<u8>) -> RgbaImage {
        let mut img = blank(40, 40);
        frame(&mut img, OUTER, (5, 5, 5, 5));
        let area = mark_area(OUTER).unwrap();
        for (dy, row) in shape.iter().enumerate() {
            for (dx, &on) in row.iter().enumerate() {
                if on {
                    img.put_pixel(area.left + dx as u32, area.top + dy as u32, color);
                }
            }
        }
        img
    }

    fn data_of(img: &RgbaImage) -> FieldData {
        let layout = detect(img);
        assert_eq!(layout.fields.len(), 1);
        layout.fields[0].data.clone()
    }

    // Mark location and shape

    #[test]
    fn mark_position_for_a_field() {
        assert_eq!(
            mark_area(Rect::new(156, 22, 193, 50)),
            Some(Rect::new(191, 19, 193, 21))
        );
    }

    #[test]
    fn no_room_above_the_field() {
        for top in 0..3 {
            assert_eq!(mark_area(Rect::new(10, top, 29, top + 19)), None);
        }
        let mut img = blank(40, 30);
        frame(&mut img, Rect::new(10, 2, 29, 21), (5, 5, 5, 5));
        assert_eq!(data_of(&img), FieldData::Image);
    }

    #[test]
    fn float_mark() {
        let img = field_with_mark(FLOAT_MARK, GREEN);
        assert!(matches!(data_of(&img), FieldData::Float { color, .. } if color == GREEN.0));
    }

    #[test]
    fn counter_mark() {
        let img = field_with_mark(COUNTER_MARK, RED);
        assert!(matches!(data_of(&img), FieldData::Counter { color, .. } if color == RED.0));
    }

    #[test]
    fn other_pixels_are_not_the_mark_color() {
        let mut img = field_with_mark(FLOAT_MARK, GREEN);
        let area = mark_area(OUTER).unwrap();
        img.put_pixel(area.right, area.top + 1, BLACK);
        img.put_pixel(area.left + 1, area.bottom, BLACK);
        assert_eq!(data_of(&img).type_name(), "float");
    }

    #[test]
    fn no_mark() {
        let img = field_with_mark([[false; 3]; 3], GREEN);
        assert_eq!(data_of(&img), FieldData::Image);
    }

    #[test]
    fn unrecognized_shape() {
        let x = [
            [true, false, true],
            [false, true, false],
            [true, false, true],
        ];
        assert_eq!(data_of(&field_with_mark(x, GREEN)), FieldData::Image);
    }

    #[test]
    fn frame_colored_mark() {
        // The mark sits right above the frame, so in the frame color it would
        // also make the frame look thicker; read the type directly instead.
        let img = field_with_mark(FLOAT_MARK, FRAME_COLOR);
        let field = Field {
            id: 1,
            outer: OUTER,
            inner: Rect::new(15, 15, 24, 24),
            borders: crate::Borders {
                top: 5,
                right: 5,
                bottom: 5,
                left: 5,
            },
            data: FieldData::Image,
        };
        assert_eq!(read_data(&img, &field), FieldData::Image);
        assert_eq!(data_of(&img), FieldData::Image);
    }

    // Float value

    #[test]
    fn float_partly_filled() {
        let mut img = field_with_mark(FLOAT_MARK, GREEN);
        fill(&mut img, Rect::new(15, 15, 24, 16), GREEN); // 20 pixels
        fill(&mut img, Rect::new(15, 17, 19, 17), GREEN); // 5 pixels
        fill(&mut img, Rect::new(15, 20, 24, 20), BLACK); // 10 pixels
        let data = data_of(&img);
        assert!(matches!(
            data,
            FieldData::Float {
                filled: 25,
                total: 100,
                ..
            }
        ));
        assert_eq!(data.percent(), Some(25.0));
    }

    #[test]
    fn float_empty_and_full() {
        let img = field_with_mark(FLOAT_MARK, GREEN);
        assert_eq!(data_of(&img).percent(), Some(0.0));
        let mut full = img.clone();
        fill(&mut full, Rect::new(15, 15, 24, 24), GREEN);
        assert_eq!(data_of(&full).percent(), Some(100.0));
    }

    // Counter value

    fn count_of(img: &RgbaImage) -> u64 {
        match data_of(img) {
            FieldData::Counter { count, .. } => count,
            other => panic!("expected a counter, got {other:?}"),
        }
    }

    #[test]
    fn counter_separate_dots() {
        let mut img = field_with_mark(COUNTER_MARK, RED);
        fill(&mut img, Rect::new(15, 15, 16, 16), RED);
        fill(&mut img, Rect::new(20, 15, 22, 17), RED);
        fill(&mut img, Rect::new(16, 21, 23, 22), RED);
        assert_eq!(count_of(&img), 3);
    }

    #[test]
    fn counter_diagonal_contact_does_not_join() {
        let mut img = field_with_mark(COUNTER_MARK, RED);
        img.put_pixel(18, 18, RED);
        img.put_pixel(19, 19, RED);
        assert_eq!(count_of(&img), 2);
    }

    #[test]
    fn counter_other_colors_do_not_count() {
        let mut img = field_with_mark(COUNTER_MARK, RED);
        fill(&mut img, Rect::new(15, 15, 16, 16), RED);
        fill(&mut img, Rect::new(22, 22, 24, 24), RED);
        for (x, y) in [(19, 15), (15, 20), (19, 19), (23, 18)] {
            img.put_pixel(x, y, BLUE);
        }
        assert_eq!(count_of(&img), 2);
    }

    #[test]
    fn counter_empty_area() {
        let img = field_with_mark(COUNTER_MARK, RED);
        assert_eq!(count_of(&img), 0);
    }

    // Bool mark and value

    fn bool_of(img: &RgbaImage) -> bool {
        match data_of(img) {
            FieldData::Bool { value, .. } => value,
            other => panic!("expected a bool, got {other:?}"),
        }
    }

    #[test]
    fn bool_mark() {
        let img = field_with_mark(BOOL_MARK, PINK);
        assert!(matches!(data_of(&img), FieldData::Bool { color, .. } if color == PINK.0));
    }

    #[test]
    fn top_left_pixel_in_another_color() {
        let mut img = field_with_mark(BOOL_MARK, PINK);
        let area = mark_area(OUTER).unwrap();
        img.put_pixel(area.left, area.top, BLACK);
        assert!(matches!(data_of(&img), FieldData::Bool { color, .. } if color == PINK.0));
    }

    #[test]
    fn string_mark() {
        let mut img = field_with_mark(STRING_MARK, BLACK);
        // "HI" in the data area (15,15)-(24,24), one empty column apart.
        let h = [(15, 15), (15, 16), (15, 17), (15, 18), (15, 19), (16, 17)];
        let h = h
            .iter()
            .chain(&[(17, 15), (17, 16), (17, 17), (17, 18), (17, 19)]);
        for &(x, y) in h {
            img.put_pixel(x, y, BLACK);
        }
        for x in 19..=21 {
            img.put_pixel(x, 15, BLACK);
            img.put_pixel(x, 19, BLACK);
        }
        for y in 16..=18 {
            img.put_pixel(20, y, BLACK);
        }
        assert_eq!(
            data_of(&img),
            FieldData::String {
                color: BLACK.0,
                text: "HI".to_string()
            }
        );
    }

    #[test]
    fn plus_shaped_mark_is_ref() {
        let plus = [
            [false, true, false],
            [true, true, true],
            [false, true, false],
        ];
        assert_eq!(plus, REF_MARK);
        assert_eq!(
            data_of(&field_with_mark(plus, YELLOW)),
            FieldData::Ref {
                color: YELLOW.0,
                record: None
            }
        );
    }

    #[test]
    fn ref_data_area_is_ignored() {
        let empty = field_with_mark(REF_MARK, YELLOW);
        let mut drawn = empty.clone();
        fill(&mut drawn, Rect::new(15, 15, 19, 19), YELLOW);
        fill(&mut drawn, Rect::new(20, 20, 24, 24), BLACK);
        assert_eq!(data_of(&drawn), data_of(&empty));
        assert_eq!(data_of(&drawn).type_name(), "ref");
    }

    #[test]
    fn bool_one_pixel_makes_it_true() {
        let mut img = field_with_mark(BOOL_MARK, PINK);
        img.put_pixel(20, 20, PINK);
        assert!(bool_of(&img));
    }

    #[test]
    fn bool_filled_area() {
        let mut img = field_with_mark(BOOL_MARK, PINK);
        fill(&mut img, Rect::new(16, 16, 23, 23), PINK);
        assert!(bool_of(&img));
    }

    #[test]
    fn bool_other_colors_only() {
        let mut img = field_with_mark(BOOL_MARK, PINK);
        fill(&mut img, Rect::new(15, 15, 18, 18), BLACK);
        fill(&mut img, Rect::new(20, 20, 24, 24), RED);
        assert!(!bool_of(&img));
    }

    #[test]
    fn bool_empty_area() {
        let img = field_with_mark(BOOL_MARK, PINK);
        assert!(!bool_of(&img));
    }
}
