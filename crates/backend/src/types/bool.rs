//! The `bool` type: true when the data area holds any pixel of the mark
//! color. Mark `.#.` / `.##` / `.##`.

use image::Rgba;

use super::{AreaView, FieldType, Input, InputKind, Mark, Value};
use crate::draw::{Plan, WriteError, full};
use crate::layout::{Field, Rect};

/// The `bool` field type.
pub struct Bool;

/// Mark shape of a `bool` field, rows top to bottom: `.#.`, `.##`, `.##`.
pub const BOOL_MARK: Mark = [
    [false, true, false],
    [false, true, true],
    [false, true, true],
];

impl FieldType for Bool {
    fn name(&self) -> &'static str {
        "bool"
    }

    fn mark(&self) -> Option<Mark> {
        Some(BOOL_MARK)
    }

    fn read(&self, area: &AreaView<'_>, color: [u8; 4]) -> Value {
        Value::Bool(area.pixels().any(|p| p == color))
    }

    fn input_kind(&self) -> Option<InputKind> {
        Some(InputKind::Bool)
    }

    /// True fills the whole data area with the mark color; false clears it.
    fn plan(&self, f: &Field, input: Input) -> Result<Plan, WriteError> {
        let Input::Bool(value) = input else {
            unreachable!("checked against input_kind");
        };
        let (w, h) = (f.inner.width(), f.inner.height());
        let pixels = if value {
            full(Rect::new(0, 0, w - 1, h - 1))
        } else {
            Vec::new()
        };
        Ok(Plan::Pixels(Rgba(f.mark_color()), pixels))
    }

    fn cli_text(&self, value: &Value) -> String {
        match value {
            Value::Bool(v) => format!("bool {v}"),
            _ => "bool".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::tests::fill;
    use crate::types::mark_area;
    use crate::types::testing::*;

    const PINK: Rgba<u8> = Rgba([255, 174, 201, 255]);
    const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);

    fn bool_of(img: &image::RgbaImage) -> bool {
        match data_of(img) {
            ("bool", Value::Bool(v)) => v,
            other => panic!("expected a bool, got {other:?}"),
        }
    }

    #[test]
    fn bool_mark() {
        let f = field_of(&field_with_mark(BOOL_MARK, PINK));
        assert_eq!((f.type_name(), f.color), ("bool", Some(PINK.0)));
    }

    #[test]
    fn top_left_pixel_in_another_color() {
        let mut img = field_with_mark(BOOL_MARK, PINK);
        let area = mark_area(OUTER).unwrap();
        img.put_pixel(area.left, area.top, BLACK);
        let f = field_of(&img);
        assert_eq!((f.type_name(), f.color), ("bool", Some(PINK.0)));
    }

    #[test]
    fn one_pixel_makes_it_true() {
        let mut img = field_with_mark(BOOL_MARK, PINK);
        img.put_pixel(20, 20, PINK);
        assert!(bool_of(&img));
    }

    #[test]
    fn filled_area() {
        let mut img = field_with_mark(BOOL_MARK, PINK);
        fill(&mut img, Rect::new(16, 16, 23, 23), PINK);
        assert!(bool_of(&img));
    }

    #[test]
    fn other_colors_only() {
        let mut img = field_with_mark(BOOL_MARK, PINK);
        fill(&mut img, Rect::new(15, 15, 18, 18), BLACK);
        fill(&mut img, Rect::new(20, 20, 24, 24), RED);
        assert!(!bool_of(&img));
    }

    #[test]
    fn empty_area() {
        assert!(!bool_of(&field_with_mark(BOOL_MARK, PINK)));
    }

    #[test]
    fn cli_text() {
        assert_eq!(Bool.cli_text(&Value::Bool(true)), "bool true");
        assert_eq!(Bool.cli_text(&Value::Bool(false)), "bool false");
    }

    #[test]
    fn true_and_false() {
        let mut img = scene(8, 11, Some(BOOL_MARK));
        assert_eq!(write(&mut img, Input::Bool(false)).unwrap(), Value::Bool(false));
        assert_eq!(write(&mut img, Input::Bool(true)).unwrap(), Value::Bool(true));
        assert_eq!(data_pixels(&img, 8, 11).len(), 88);
    }
}
