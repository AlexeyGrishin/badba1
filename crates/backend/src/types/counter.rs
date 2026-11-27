//! The `counter` type: the number of separate objects in the data area.
//! Mark `###` / `#..` / `###`.

use std::collections::VecDeque;

use image::Rgba;

use super::{AreaView, FieldType, Input, InputKind, Mark, Value};
use crate::draw::{Plan, WriteError, full};
use crate::layout::{Field, Rect};

/// The `counter` field type.
pub struct Counter;

/// Mark shape of a `counter` field, rows top to bottom: `###`, `#..`, `###`.
pub const COUNTER_MARK: Mark = [[true, true, true], [true, false, false], [true, true, true]];

impl FieldType for Counter {
    fn name(&self) -> &'static str {
        "counter"
    }

    fn mark(&self) -> Option<Mark> {
        Some(COUNTER_MARK)
    }

    /// Counts groups of mark-color pixels connected horizontally or
    /// vertically.
    fn read(&self, area: &AreaView<'_>, color: [u8; 4]) -> Value {
        let (w, h) = (area.width() as usize, area.height() as usize);
        let is_color = |x: usize, y: usize| area.get(x as u32, y as u32) == color;
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
        Value::Int(count)
    }

    fn input_kind(&self) -> Option<InputKind> {
        Some(InputKind::Int)
    }

    /// N squares of the largest size that fits on a grid with 1-pixel gaps,
    /// in reading order.
    fn plan(&self, f: &Field, input: Input) -> Result<Plan, WriteError> {
        let Input::Int(n) = input else {
            unreachable!("checked against input_kind");
        };
        let (w, h) = (f.inner.width(), f.inner.height());
        let color = Rgba(f.mark_color());
        if n == 0 {
            return Ok(Plan::Pixels(color, Vec::new()));
        }
        let capacity = |s: u32| u64::from((w + 1) / (s + 1)) * u64::from((h + 1) / (s + 1));
        if capacity(1) < n {
            return Err(WriteError::FieldOverflow {
                field: f.id,
                reason: format!(
                    "{w}x{h} data area holds at most {} squares, not {n}",
                    capacity(1)
                ),
            });
        }
        let s = (1..=w.min(h))
            .take_while(|&s| capacity(s) >= n)
            .last()
            .unwrap_or(1);
        let cols = u64::from((w + 1) / (s + 1));
        let mut pixels = Vec::new();
        for i in 0..n {
            let (c, r) = ((i % cols) as u32, (i / cols) as u32);
            let (x0, y0) = (c * (s + 1), r * (s + 1));
            pixels.extend(full(Rect::new(x0, y0, x0 + s - 1, y0 + s - 1)));
        }
        Ok(Plan::Pixels(color, pixels))
    }

    fn cli_text(&self, value: &Value) -> String {
        match value {
            Value::Int(n) => format!("counter {n}"),
            _ => "counter".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::tests::fill;
    use crate::types::testing::*;

    const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);

    fn count_of(img: &image::RgbaImage) -> u64 {
        match data_of(img) {
            ("counter", Value::Int(n)) => n,
            other => panic!("expected a counter, got {other:?}"),
        }
    }

    #[test]
    fn counter_mark() {
        let f = field_of(&field_with_mark(COUNTER_MARK, RED));
        assert_eq!((f.type_name(), f.color), ("counter", Some(RED.0)));
    }

    #[test]
    fn separate_dots() {
        let mut img = field_with_mark(COUNTER_MARK, RED);
        fill(&mut img, Rect::new(15, 15, 16, 16), RED);
        fill(&mut img, Rect::new(20, 15, 22, 17), RED);
        fill(&mut img, Rect::new(16, 21, 23, 22), RED);
        assert_eq!(count_of(&img), 3);
    }

    #[test]
    fn diagonal_contact_does_not_join() {
        let mut img = field_with_mark(COUNTER_MARK, RED);
        img.put_pixel(18, 18, RED);
        img.put_pixel(19, 19, RED);
        assert_eq!(count_of(&img), 2);
    }

    #[test]
    fn other_colors_do_not_count() {
        let mut img = field_with_mark(COUNTER_MARK, RED);
        fill(&mut img, Rect::new(15, 15, 16, 16), RED);
        fill(&mut img, Rect::new(22, 22, 24, 24), RED);
        for (x, y) in [(19, 15), (15, 20), (19, 19), (23, 18)] {
            img.put_pixel(x, y, BLUE);
        }
        assert_eq!(count_of(&img), 2);
    }

    #[test]
    fn empty_area() {
        assert_eq!(count_of(&field_with_mark(COUNTER_MARK, RED)), 0);
    }

    #[test]
    fn cli_text() {
        assert_eq!(Counter.cli_text(&Value::Int(5)), "counter 5");
    }

    #[test]
    fn squares_in_reading_order() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        assert_eq!(write(&mut img, Input::Int(3)).unwrap(), Value::Int(3));
        // s = 8: three 8x8 squares at x 0, 9, 18 on the first row.
        let px = data_pixels(&img, 27, 16);
        assert_eq!(px.len(), 3 * 64);
        assert!(px.iter().all(|&(x, y)| y < 8 && x % 9 < 8));
    }

    #[test]
    fn zero_clears() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        fill(&mut img, Rect::new(15, 15, 18, 18), RED);
        assert_eq!(write(&mut img, Input::Int(0)).unwrap(), Value::Int(0));
        assert!(data_pixels(&img, 27, 16).is_empty());
    }

    #[test]
    fn largest_that_fits() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        assert_eq!(write(&mut img, Input::Int(112)).unwrap(), Value::Int(112));
        assert_eq!(data_pixels(&img, 27, 16).len(), 112);
    }

    #[test]
    fn overflow() {
        let err = fails(scene(27, 16, Some(COUNTER_MARK)), Input::Int(113));
        assert!(matches!(err, WriteError::FieldOverflow { .. }));
    }

    #[test]
    fn replaces_old_content() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        for i in 0..5 {
            fill(&mut img, Rect::new(14 + 4 * i, 20, 15 + 4 * i, 21), RED);
        }
        assert_eq!(write(&mut img, Input::Int(2)).unwrap(), Value::Int(2));
    }
}
