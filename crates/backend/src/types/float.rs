//! The `float` type: the filled share of the data area, in percent. Mark
//! `###` / `##.` / `#..`.

use image::Rgba;

use super::{AreaView, FieldType, Input, InputKind, Mark, Value};
use crate::draw::{Plan, WriteError};
use crate::layout::Field;

/// The `float` field type.
pub struct Float;

/// Mark shape of a `float` field, rows top to bottom: `###`, `##.`, `#..`.
pub const FLOAT_MARK: Mark = [
    [true, true, true],
    [true, true, false],
    [true, false, false],
];

impl FieldType for Float {
    fn name(&self) -> &'static str {
        "float"
    }

    fn mark(&self) -> Option<Mark> {
        Some(FLOAT_MARK)
    }

    /// The percentage (0 to 100) of data-area pixels in the mark color.
    fn read(&self, area: &AreaView<'_>, color: [u8; 4]) -> Value {
        let filled = area.pixels().filter(|p| *p == color).count() as u64;
        let total = u64::from(area.width()) * u64::from(area.height());
        Value::Number(if total > 0 {
            100.0 * filled as f64 / total as f64
        } else {
            0.0
        })
    }

    fn input_kind(&self) -> Option<InputKind> {
        Some(InputKind::Number)
    }

    /// Fills round(n × area / 100) pixels, column by column from the left
    /// and top to bottom within a column.
    fn plan(&self, f: &Field, input: Input) -> Result<Plan, WriteError> {
        let Input::Number(n) = input else {
            unreachable!("checked against input_kind");
        };
        if !n.is_finite() || !(0.0..=100.0).contains(&n) {
            return Err(WriteError::OutOfRange {
                field: f.id,
                value: n,
            });
        }
        let (w, h) = (f.inner.width(), f.inner.height());
        let area = u64::from(w) * u64::from(h);
        let k = (n * area as f64 / 100.0).round() as u64;
        let pixels = (0..k)
            .map(|i| ((i / u64::from(h)) as u32, (i % u64::from(h)) as u32))
            .collect();
        Ok(Plan::Pixels(Rgba(f.mark_color()), pixels))
    }

    fn cli_text(&self, value: &Value) -> String {
        let pct = match value {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        format!("float {pct:.2}%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::tests::fill;
    use crate::layout::Rect;
    use crate::types::mark_area;
    use crate::types::testing::*;

    const GREEN: Rgba<u8> = Rgba([34, 177, 76, 255]);
    const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);

    fn pct(img: &image::RgbaImage) -> f64 {
        match data_of(img) {
            ("float", Value::Number(n)) => n,
            other => panic!("expected a float, got {other:?}"),
        }
    }

    #[test]
    fn float_mark() {
        let img = field_with_mark(FLOAT_MARK, GREEN);
        let f = field_of(&img);
        assert_eq!((f.type_name(), f.color), ("float", Some(GREEN.0)));
    }

    #[test]
    fn other_pixels_are_not_the_mark_color() {
        let mut img = field_with_mark(FLOAT_MARK, GREEN);
        let area = mark_area(OUTER).unwrap();
        img.put_pixel(area.right, area.top + 1, BLACK);
        img.put_pixel(area.left + 1, area.bottom, BLACK);
        assert_eq!(data_of(&img).0, "float");
    }

    #[test]
    fn partly_filled() {
        let mut img = field_with_mark(FLOAT_MARK, GREEN);
        fill(&mut img, Rect::new(15, 15, 24, 16), GREEN); // 20 pixels
        fill(&mut img, Rect::new(15, 17, 19, 17), GREEN); // 5 pixels
        fill(&mut img, Rect::new(15, 20, 24, 20), BLACK); // 10 pixels
        assert_eq!(pct(&img), 25.0);
    }

    #[test]
    fn empty_and_full() {
        let img = field_with_mark(FLOAT_MARK, GREEN);
        assert_eq!(pct(&img), 0.0);
        let mut full = img.clone();
        fill(&mut full, Rect::new(15, 15, 24, 24), GREEN);
        assert_eq!(pct(&full), 100.0);
    }

    #[test]
    fn cli_text() {
        assert_eq!(Float.cli_text(&Value::Number(100.0 * 196.0 / 532.0)), "float 36.84%");
        assert_eq!(Float.cli_text(&Value::Number(0.0)), "float 0.00%");
    }

    #[test]
    fn stored_value_differs_from_requested() {
        let mut img = scene(28, 19, Some(FLOAT_MARK));
        let got = write(&mut img, Input::Number(33.3)).unwrap();
        assert_eq!(got, Value::Number(100.0 * 177.0 / 532.0));
        // Column-major from the left: the first 177 pixels fill 9 full
        // columns of 19 and 6 pixels of the tenth.
        let px = data_pixels(&img, 28, 19);
        assert_eq!(px.len(), 177);
        assert!(px.iter().all(|&(x, y)| x < 9 || (x == 9 && y < 6)));
    }

    #[test]
    fn exact_value() {
        let mut img = scene(28, 19, Some(FLOAT_MARK));
        let got = write(&mut img, Input::Number(50.0)).unwrap();
        assert_eq!(data_pixels(&img, 28, 19).len(), 266);
        assert_eq!(Float.cli_text(&got), "float 50.00%");
    }

    #[test]
    fn out_of_range() {
        for v in [-1.0, 100.5, f64::NAN] {
            let err = fails(scene(28, 19, Some(FLOAT_MARK)), Input::Number(v));
            assert!(matches!(err, WriteError::OutOfRange { .. }), "{v}");
        }
    }
}
