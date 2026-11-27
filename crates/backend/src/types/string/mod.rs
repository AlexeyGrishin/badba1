//! The `string` type: text written in the data area with the built-in glyph
//! font. Mark `.#.` / `#.#` / `#.#`.

mod read;
mod write;

use image::Rgba;

pub use read::UNKNOWN_GLYPH;

use super::{AreaView, FieldType, Input, InputKind, Mark, Value};
use crate::draw::{Plan, WriteError};
use crate::font::builtin;
use crate::layout::Field;

/// The `string` field type.
pub struct Str;

/// Mark shape of a `string` field, rows top to bottom: `.#.`, `#.#`, `#.#`.
pub const STRING_MARK: Mark = [
    [false, true, false],
    [true, false, true],
    [true, false, true],
];

impl FieldType for Str {
    fn name(&self) -> &'static str {
        "string"
    }

    fn mark(&self) -> Option<Mark> {
        Some(STRING_MARK)
    }

    /// The text in the data area: every non-background pixel is ink.
    fn read(&self, area: &AreaView<'_>, _color: [u8; 4]) -> Value {
        Value::Text(read::read_text(area, builtin()))
    }

    fn input_kind(&self) -> Option<InputKind> {
        Some(InputKind::Text)
    }

    fn plan(&self, f: &Field, input: Input) -> Result<Plan, WriteError> {
        let Input::Text(text) = input else {
            unreachable!("checked against input_kind");
        };
        let (w, h) = (f.inner.width(), f.inner.height());
        let pixels = write::string_pixels(f.id, w, h, &text)?;
        Ok(Plan::Pixels(Rgba(f.mark_color()), pixels))
    }

    fn cli_text(&self, value: &Value) -> String {
        match value {
            Value::Text(text) => format!("string {text:?}"),
            _ => "string".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::testing::*;

    const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);

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
        let f = field_of(&img);
        assert_eq!(f.color, Some(BLACK.0));
        assert_eq!(data_of(&img), ("string", Value::Text("HI".to_string())));
    }

    #[test]
    fn cli_text() {
        assert_eq!(
            Str.cli_text(&Value::Text("OPENSPEC".into())),
            "string \"OPENSPEC\""
        );
        assert_eq!(Str.cli_text(&Value::Text("A\nB".into())), r#"string "A\nB""#);
    }
}
