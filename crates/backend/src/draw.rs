//! Writing values into fields' data areas, for every type.
//!
//! A write validates the value and plans every pixel first (the field type's
//! [`plan`](crate::types::FieldType::plan)); only then does it clear the data
//! area to the background color (the image's top-left pixel) and draw. So a
//! failed write never changes the image. The returned value is read back from
//! the new pixels.

use std::fmt;

use image::{Rgba, RgbaImage};

use crate::layout::{Field, Layout, Rect};
use crate::types::{AreaView, Input, Value, name_for_input};

/// Why a write failed. The image is unchanged in every case.
#[derive(Debug, Clone, PartialEq)]
pub enum WriteError {
    /// The layout has no field with this number.
    NoSuchField(usize),
    /// The field's type cannot be written (references).
    NotWritable {
        /// The field's number.
        field: usize,
        /// The field's type.
        type_name: &'static str,
    },
    /// The value is for a different type than the field's.
    TypeMismatch {
        /// The field's number.
        field: usize,
        /// The field's type.
        field_type: &'static str,
        /// The type the value is for.
        value_type: &'static str,
    },
    /// The value does not fit in the field's data area.
    FieldOverflow {
        /// The field's number.
        field: usize,
        /// Why it does not fit.
        reason: String,
    },
    /// A float value outside 0 to 100, or not a number.
    OutOfRange {
        /// The field's number.
        field: usize,
        /// The value.
        value: f64,
    },
    /// A character the glyph font does not have.
    UnknownCharacter {
        /// The field's number.
        field: usize,
        /// The character.
        character: char,
    },
    /// The bitmap holds the frame color at (x, y).
    FrameColor {
        /// The field's number.
        field: usize,
        /// Column in the bitmap.
        x: u32,
        /// Row in the bitmap.
        y: u32,
    },
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::NoSuchField(id) => write!(f, "no field F{id}"),
            WriteError::NotWritable { field, type_name } => {
                write!(
                    f,
                    "field F{field} is a {type_name} field and cannot be written"
                )
            }
            WriteError::TypeMismatch {
                field,
                field_type,
                value_type,
            } => write!(
                f,
                "field F{field} is a {field_type} field, not a {value_type} field"
            ),
            WriteError::FieldOverflow { field, reason } => {
                write!(f, "value does not fit in field F{field}: {reason}")
            }
            WriteError::OutOfRange { field, value } => {
                write!(
                    f,
                    "value {value} for field F{field} is not between 0 and 100"
                )
            }
            WriteError::UnknownCharacter { field, character } => write!(
                f,
                "character {character:?} for field F{field} is not in the glyph font"
            ),
            WriteError::FrameColor { field, x, y } => write!(
                f,
                "image for field F{field} has the frame color at ({x},{y})"
            ),
        }
    }
}

impl std::error::Error for WriteError {}

/// What to draw into a cleared data area.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Pixels relative to the data area's top-left, in one color.
    Pixels(Rgba<u8>, Vec<(u32, u32)>),
    /// A bitmap placed at the data area's top-left.
    Bitmap(RgbaImage),
}

/// All pixels of `r`.
pub(crate) fn full(r: Rect) -> Vec<(u32, u32)> {
    (r.top..=r.bottom)
        .flat_map(|y| (r.left..=r.right).map(move |x| (x, y)))
        .collect()
}

/// Validates `input` for field `f` and works out what to draw: fails when
/// the field's type cannot be written or takes another kind of input, and
/// otherwise asks the type.
pub fn plan(f: &Field, input: Input) -> Result<Plan, WriteError> {
    let Some(kind) = f.kind.input_kind() else {
        return Err(WriteError::NotWritable {
            field: f.id,
            type_name: f.type_name(),
        });
    };
    if input.kind() != kind {
        return Err(WriteError::TypeMismatch {
            field: f.id,
            field_type: f.type_name(),
            value_type: name_for_input(input.kind()),
        });
    }
    f.kind.plan(f, input)
}

/// The new pixels of a `width`×`height` data area: `background`, with
/// `plan` drawn over it.
pub fn render(plan: &Plan, width: u32, height: u32, background: Rgba<u8>) -> RgbaImage {
    let mut area = RgbaImage::from_pixel(width, height, background);
    match plan {
        Plan::Pixels(color, pixels) => {
            for &(x, y) in pixels {
                area.put_pixel(x, y, *color);
            }
        }
        Plan::Bitmap(bitmap) => {
            for (x, y, p) in bitmap.enumerate_pixels() {
                area.put_pixel(x, y, *p);
            }
        }
    }
    area
}

/// The value of field `f` read back from `area`, its new data-area pixels.
pub(crate) fn read_back(f: &Field, area: &RgbaImage, background: Rgba<u8>) -> Value {
    let view = AreaView::whole(area, background.0);
    f.kind.read(&view, f.color.unwrap_or(background.0))
}

/// Writes `input` into field number `field` of `image`, whose detected layout
/// is `layout`, and returns the field's value read back from the result.
pub fn write_field(
    image: &mut RgbaImage,
    layout: &Layout,
    field: usize,
    input: Input,
) -> Result<Value, WriteError> {
    let f = layout
        .fields
        .iter()
        .find(|f| f.id == field)
        .ok_or(WriteError::NoSuchField(field))?;
    let plan = plan(f, input)?;
    let background = *image.get_pixel(0, 0);
    let inner = f.inner;
    let area = render(&plan, inner.width(), inner.height(), background);
    for (x, y, p) in area.enumerate_pixels() {
        image.put_pixel(inner.left + x, inner.top + y, *p);
    }
    Ok(read_back(f, &area, background))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::detect;
    use crate::types::testing::*;
    use crate::types::{by_name, mark_area};

    fn mark(name: &str) -> Option<crate::types::Mark> {
        by_name(name).unwrap().mark()
    }

    #[test]
    fn unknown_field() {
        let mut img = scene(27, 16, mark("counter"));
        let before = img.clone();
        let layout = detect(&img);
        let err = write_field(&mut img, &layout, 99, Input::Int(1)).unwrap_err();
        assert_eq!(err, WriteError::NoSuchField(99));
        assert!(err.to_string().contains("99"));
        assert_eq!(img, before);
    }

    #[test]
    fn type_mismatch_names_both_types() {
        let err = fails(scene(27, 16, mark("counter")), Input::Text("A".into()));
        assert_eq!(
            err.to_string(),
            "field F1 is a counter field, not a string field"
        );
    }

    #[test]
    fn not_writable_is_checked_before_the_input() {
        let err = fails(scene(10, 10, mark("ref")), Input::Text("A".into()));
        assert!(matches!(err, WriteError::NotWritable { .. }));
    }

    #[test]
    fn pixels_outside_the_data_area_are_untouched() {
        let mut img = scene(27, 16, mark("counter"));
        let before = img.clone();
        write(&mut img, Input::Int(3)).unwrap();
        for (x, y, p) in img.enumerate_pixels() {
            if !Rect::new(13, 13, 39, 28).contains(x, y) {
                assert_eq!(p, before.get_pixel(x, y), "pixel ({x},{y}) changed");
            }
        }
        // The mark is still there.
        let area = mark_area(Rect::new(10, 10, 42, 31)).unwrap();
        assert_eq!(*img.get_pixel(area.left + 1, area.top), RED);
    }

    #[test]
    fn layout_is_unchanged_by_a_write() {
        let mut img = scene(27, 16, mark("counter"));
        let before = detect(&img);
        assert_eq!(write(&mut img, Input::Int(3)).unwrap(), Value::Int(3));
        let after = detect(&img);
        assert_eq!(after.fields.len(), before.fields.len());
        assert_eq!(after.fields[0].outer, before.fields[0].outer);
        assert_eq!(after.fields[0].value, Value::Int(3));
    }
}
