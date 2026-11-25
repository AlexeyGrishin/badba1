//! Writing values into fields' data areas.
//!
//! A write validates the value and plans every pixel first; only then does it
//! clear the data area to the background color (the image's top-left pixel)
//! and draw. So a failed write never changes the image. The returned value is
//! read back from the modified image.

use std::fmt;

use image::{Rgba, RgbaImage};

use crate::detect::FRAME_COLOR;
use crate::font::{GLYPH_HEIGHT, GLYPH_WIDTH, builtin, glyph_bit};
use crate::layout::{Field, FieldData, Layout, Rect};
use crate::types::read_data;

/// A value to write into a field. Its kind must match the field's type.
#[derive(Debug, Clone, PartialEq)]
pub enum WriteValue {
    /// Number of objects for a `counter` field.
    Counter(u64),
    /// Percentage (0 to 100) for a `float` field.
    Float(f64),
    /// Value of a `bool` field.
    Bool(bool),
    /// Text for a `string` field; `\n` starts a new line.
    String(String),
    /// Bitmap for an `image` field, no larger than its data area.
    Image(RgbaImage),
}

impl WriteValue {
    /// The field type this value is for.
    pub fn type_name(&self) -> &'static str {
        match self {
            WriteValue::Counter(_) => "counter",
            WriteValue::Float(_) => "float",
            WriteValue::Bool(_) => "bool",
            WriteValue::String(_) => "string",
            WriteValue::Image(_) => "image",
        }
    }
}

/// Why a write failed. The image is unchanged in every case.
#[derive(Debug, Clone, PartialEq)]
pub enum WriteError {
    /// The layout has no field with this number.
    NoSuchField(usize),
    /// The field's type cannot be written (references).
    NotWritable {
        field: usize,
        type_name: &'static str,
    },
    /// The value is for a different type than the field's.
    TypeMismatch {
        field: usize,
        field_type: &'static str,
        value_type: &'static str,
    },
    /// The value does not fit in the field's data area.
    FieldOverflow { field: usize, reason: String },
    /// A float value outside 0 to 100, or not a number.
    OutOfRange { field: usize, value: f64 },
    /// A character the glyph font does not have.
    UnknownCharacter { field: usize, character: char },
    /// The bitmap holds the frame color at (x, y).
    FrameColor { field: usize, x: u32, y: u32 },
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
enum Plan {
    /// Pixels relative to the data area's top-left, in one color.
    Pixels(Rgba<u8>, Vec<(u32, u32)>),
    /// A bitmap placed at the data area's top-left.
    Bitmap(RgbaImage),
}

/// Writes `value` into field number `field` of `image`, whose detected layout
/// is `layout`, and returns the field's value read back from the result.
pub fn write_field(
    image: &mut RgbaImage,
    layout: &Layout,
    field: usize,
    value: WriteValue,
) -> Result<FieldData, WriteError> {
    let f = layout
        .fields
        .iter()
        .find(|f| f.id == field)
        .ok_or(WriteError::NoSuchField(field))?;
    let plan = plan(f, value)?;

    let background = *image.get_pixel(0, 0);
    let inner = f.inner;
    for y in inner.top..=inner.bottom {
        for x in inner.left..=inner.right {
            image.put_pixel(x, y, background);
        }
    }
    match plan {
        Plan::Pixels(color, pixels) => {
            for (x, y) in pixels {
                image.put_pixel(inner.left + x, inner.top + y, color);
            }
        }
        Plan::Bitmap(bitmap) => {
            for (x, y, p) in bitmap.enumerate_pixels() {
                image.put_pixel(inner.left + x, inner.top + y, *p);
            }
        }
    }
    Ok(read_data(image, f))
}

/// Validates `value` for field `f` and works out what to draw.
fn plan(f: &Field, value: WriteValue) -> Result<Plan, WriteError> {
    let field = f.id;
    let color = match &f.data {
        FieldData::Ref { .. } => {
            return Err(WriteError::NotWritable {
                field,
                type_name: "ref",
            });
        }
        FieldData::Image => None,
        FieldData::Float { color, .. }
        | FieldData::Counter { color, .. }
        | FieldData::Bool { color, .. }
        | FieldData::String { color, .. } => Some(Rgba(*color)),
    };
    if f.data.type_name() != value.type_name() {
        return Err(WriteError::TypeMismatch {
            field,
            field_type: f.data.type_name(),
            value_type: value.type_name(),
        });
    }
    let (w, h) = (f.inner.width(), f.inner.height());
    // Every type but `image` has a mark color, checked just above.
    let mark = || color.expect("typed field has a mark color");
    match value {
        WriteValue::Counter(n) => Ok(Plan::Pixels(mark(), counter_pixels(field, w, h, n)?)),
        WriteValue::Float(n) => {
            if !n.is_finite() || !(0.0..=100.0).contains(&n) {
                return Err(WriteError::OutOfRange { field, value: n });
            }
            let area = u64::from(w) * u64::from(h);
            let k = (n * area as f64 / 100.0).round() as u64;
            let pixels = (0..k)
                .map(|i| ((i / u64::from(h)) as u32, (i % u64::from(h)) as u32))
                .collect();
            Ok(Plan::Pixels(mark(), pixels))
        }
        WriteValue::Bool(value) => {
            let pixels = if value {
                full(Rect::new(0, 0, w - 1, h - 1))
            } else {
                Vec::new()
            };
            Ok(Plan::Pixels(mark(), pixels))
        }
        WriteValue::String(text) => Ok(Plan::Pixels(mark(), string_pixels(field, w, h, &text)?)),
        WriteValue::Image(bitmap) => {
            let (bw, bh) = bitmap.dimensions();
            if bw > w || bh > h {
                return Err(WriteError::FieldOverflow {
                    field,
                    reason: format!("image is {bw}x{bh}, data area is {w}x{h}"),
                });
            }
            if let Some((x, y, _)) = bitmap
                .enumerate_pixels()
                .find(|(_, _, p)| **p == FRAME_COLOR)
            {
                return Err(WriteError::FrameColor { field, x, y });
            }
            Ok(Plan::Bitmap(bitmap))
        }
    }
}

/// All pixels of `r`.
fn full(r: Rect) -> Vec<(u32, u32)> {
    (r.top..=r.bottom)
        .flat_map(|y| (r.left..=r.right).map(move |x| (x, y)))
        .collect()
}

/// N squares of the largest size that fits on a grid with 1-pixel gaps, in
/// reading order.
fn counter_pixels(field: usize, w: u32, h: u32, n: u64) -> Result<Vec<(u32, u32)>, WriteError> {
    if n == 0 {
        return Ok(Vec::new());
    }
    let capacity = |s: u32| u64::from((w + 1) / (s + 1)) * u64::from((h + 1) / (s + 1));
    if capacity(1) < n {
        return Err(WriteError::FieldOverflow {
            field,
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
    Ok(pixels)
}

/// Glyph pixels for `text`: 1-column letter gap, 3-column space, `\n` moves
/// down 6 rows to the left edge, no wrapping.
fn string_pixels(field: usize, w: u32, h: u32, text: &str) -> Result<Vec<(u32, u32)>, WriteError> {
    let font = builtin();
    let (mut x, mut y) = (0u32, 0u32);
    let mut pixels = Vec::new();
    for ch in text.chars() {
        match ch {
            '\n' => {
                x = 0;
                y += GLYPH_HEIGHT + 1;
            }
            ' ' => x += GLYPH_WIDTH,
            _ => {
                let mask = font.main_form(ch).ok_or(WriteError::UnknownCharacter {
                    field,
                    character: ch,
                })?;
                if x + GLYPH_WIDTH > w || y + GLYPH_HEIGHT > h {
                    return Err(WriteError::FieldOverflow {
                        field,
                        reason: format!(
                            "letter {ch:?} at ({x},{y}) does not fit in the {w}x{h} data area"
                        ),
                    });
                }
                for cy in 0..GLYPH_HEIGHT {
                    for cx in 0..GLYPH_WIDTH {
                        if mask & glyph_bit(cx, cy) != 0 {
                            pixels.push((x + cx, y + cy));
                        }
                    }
                }
                x += GLYPH_WIDTH + 1;
            }
        }
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::detect;
    use crate::detect::tests::{blank, fill, frame};
    use crate::types::{BOOL_MARK, COUNTER_MARK, FLOAT_MARK, REF_MARK, STRING_MARK, mark_area};

    const RED: Rgba<u8> = Rgba([237, 28, 36, 255]);
    const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);

    /// An image with one field whose data area is `w`×`h` at (13,13), framed
    /// with 3px borders, and the given mark (or none).
    fn scene(w: u32, h: u32, mark: Option<[[bool; 3]; 3]>) -> RgbaImage {
        let outer = Rect::new(10, 10, 10 + w + 5, 10 + h + 5);
        let mut img = blank(outer.right + 10, outer.bottom + 10);
        frame(&mut img, outer, (3, 3, 3, 3));
        if let Some(shape) = mark {
            let area = mark_area(outer).unwrap();
            for (dy, row) in shape.iter().enumerate() {
                for (dx, &on) in row.iter().enumerate() {
                    if on {
                        img.put_pixel(area.left + dx as u32, area.top + dy as u32, RED);
                    }
                }
            }
        }
        img
    }

    /// Writes into the scene's single field (F1).
    fn write(img: &mut RgbaImage, value: WriteValue) -> Result<FieldData, WriteError> {
        let layout = detect(img);
        assert_eq!(layout.fields.len(), 1);
        write_field(img, &layout, 1, value)
    }

    /// Asserts that `value` fails on the scene with `err`, leaving it unchanged.
    fn fails(mut img: RgbaImage, value: WriteValue) -> WriteError {
        let before = img.clone();
        let err = write(&mut img, value).unwrap_err();
        assert_eq!(img, before, "image changed by a failed write");
        err
    }

    fn data_pixels(img: &RgbaImage, w: u32, h: u32) -> Vec<(u32, u32)> {
        let mut v = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if *img.get_pixel(13 + x, 13 + y) != WHITE {
                    v.push((x, y));
                }
            }
        }
        v
    }

    // Shared rules

    #[test]
    fn unknown_field() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        let before = img.clone();
        let layout = detect(&img);
        let err = write_field(&mut img, &layout, 99, WriteValue::Counter(1)).unwrap_err();
        assert_eq!(err, WriteError::NoSuchField(99));
        assert!(err.to_string().contains("99"));
        assert_eq!(img, before);
    }

    #[test]
    fn reference_field_is_not_writable() {
        let err = fails(scene(10, 10, Some(REF_MARK)), WriteValue::Counter(1));
        assert!(matches!(
            err,
            WriteError::NotWritable {
                type_name: "ref",
                ..
            }
        ));
    }

    #[test]
    fn type_mismatch() {
        let err = fails(
            scene(27, 16, Some(COUNTER_MARK)),
            WriteValue::String("A".into()),
        );
        let text = err.to_string();
        assert!(
            text.contains("counter") && text.contains("string"),
            "{text}"
        );
    }

    #[test]
    fn pixels_outside_the_data_area_are_untouched() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        let before = img.clone();
        write(&mut img, WriteValue::Counter(3)).unwrap();
        for (x, y, p) in img.enumerate_pixels() {
            if !Rect::new(13, 13, 39, 28).contains(x, y) {
                assert_eq!(p, before.get_pixel(x, y), "pixel ({x},{y}) changed");
            }
        }
    }

    // Counter

    #[test]
    fn counter_squares_in_reading_order() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        let got = write(&mut img, WriteValue::Counter(3)).unwrap();
        assert!(matches!(got, FieldData::Counter { count: 3, .. }));
        // s = 8: three 8x8 squares at x 0, 9, 18 on the first row.
        let px = data_pixels(&img, 27, 16);
        assert_eq!(px.len(), 3 * 64);
        assert!(px.iter().all(|&(x, y)| y < 8 && x % 9 < 8));
    }

    #[test]
    fn counter_zero_clears() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        fill(&mut img, Rect::new(15, 15, 18, 18), RED);
        let got = write(&mut img, WriteValue::Counter(0)).unwrap();
        assert!(matches!(got, FieldData::Counter { count: 0, .. }));
        assert!(data_pixels(&img, 27, 16).is_empty());
    }

    #[test]
    fn counter_largest_that_fits() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        let got = write(&mut img, WriteValue::Counter(112)).unwrap();
        assert!(matches!(got, FieldData::Counter { count: 112, .. }));
        assert_eq!(data_pixels(&img, 27, 16).len(), 112);
    }

    #[test]
    fn counter_overflow() {
        let err = fails(scene(27, 16, Some(COUNTER_MARK)), WriteValue::Counter(113));
        assert!(matches!(err, WriteError::FieldOverflow { .. }));
    }

    #[test]
    fn counter_replaces_old_content() {
        let mut img = scene(27, 16, Some(COUNTER_MARK));
        for i in 0..5 {
            fill(&mut img, Rect::new(14 + 4 * i, 20, 15 + 4 * i, 21), RED);
        }
        let got = write(&mut img, WriteValue::Counter(2)).unwrap();
        assert!(matches!(got, FieldData::Counter { count: 2, .. }));
    }

    // Float

    #[test]
    fn float_stored_value_differs() {
        let mut img = scene(28, 19, Some(FLOAT_MARK));
        let got = write(&mut img, WriteValue::Float(33.3)).unwrap();
        assert!(matches!(
            got,
            FieldData::Float {
                filled: 177,
                total: 532,
                ..
            }
        ));
        assert!((got.percent().unwrap() - 33.27).abs() < 0.01);
        // Column-major from the left: the first 177 pixels fill 9 full
        // columns of 19 and 6 pixels of the tenth.
        let px = data_pixels(&img, 28, 19);
        assert!(px.iter().all(|&(x, y)| x < 9 || (x == 9 && y < 6)));
    }

    #[test]
    fn float_exact_value() {
        let mut img = scene(28, 19, Some(FLOAT_MARK));
        let got = write(&mut img, WriteValue::Float(50.0)).unwrap();
        assert!(matches!(got, FieldData::Float { filled: 266, .. }));
        assert_eq!(format!("{:.2}", got.percent().unwrap()), "50.00");
    }

    #[test]
    fn float_out_of_range() {
        for v in [-1.0, 100.5, f64::NAN] {
            let err = fails(scene(28, 19, Some(FLOAT_MARK)), WriteValue::Float(v));
            assert!(matches!(err, WriteError::OutOfRange { .. }), "{v}");
        }
    }

    // Bool

    #[test]
    fn bool_true_and_false() {
        let mut img = scene(8, 11, Some(BOOL_MARK));
        let got = write(&mut img, WriteValue::Bool(false)).unwrap();
        assert!(matches!(got, FieldData::Bool { value: false, .. }));
        let got = write(&mut img, WriteValue::Bool(true)).unwrap();
        assert!(matches!(got, FieldData::Bool { value: true, .. }));
        assert_eq!(data_pixels(&img, 8, 11).len(), 88);
    }

    // String

    fn text_of(data: FieldData) -> String {
        match data {
            FieldData::String { text, .. } => text,
            other => panic!("expected a string, got {other:?}"),
        }
    }

    #[test]
    fn string_one_line() {
        let mut img = scene(40, 5, Some(STRING_MARK));
        let got = write(&mut img, WriteValue::String("HELLO".into())).unwrap();
        assert_eq!(text_of(got), "HELLO");
        let px = data_pixels(&img, 40, 5);
        let starts: std::collections::BTreeSet<u32> = px.iter().map(|&(x, _)| x - x % 4).collect();
        assert_eq!(
            starts.into_iter().collect::<Vec<_>>(),
            vec![0, 4, 8, 12, 16]
        );
    }

    #[test]
    fn string_space() {
        let mut img = scene(40, 5, Some(STRING_MARK));
        let got = write(&mut img, WriteValue::String("A B".into())).unwrap();
        assert_eq!(text_of(got), "A B");
        let cols: std::collections::BTreeSet<u32> =
            data_pixels(&img, 40, 5).iter().map(|&(x, _)| x).collect();
        assert_eq!(cols.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 7, 8, 9]);
    }

    #[test]
    fn string_two_lines() {
        let mut img = scene(40, 11, Some(STRING_MARK));
        let got = write(&mut img, WriteValue::String("AB\nCD".into())).unwrap();
        assert_eq!(text_of(got), "AB\nCD");
        assert!(
            data_pixels(&img, 40, 11)
                .iter()
                .any(|&(x, y)| x == 0 && y >= 6)
        );
    }

    #[test]
    fn string_unknown_character() {
        let err = fails(
            scene(40, 5, Some(STRING_MARK)),
            WriteValue::String("Hi".into()),
        );
        assert!(matches!(
            err,
            WriteError::UnknownCharacter { character: 'i', .. }
        ));
    }

    #[test]
    fn string_too_wide() {
        let err = fails(
            scene(40, 5, Some(STRING_MARK)),
            WriteValue::String("HELLO WORLD".into()),
        );
        assert!(matches!(err, WriteError::FieldOverflow { .. }));
    }

    #[test]
    fn string_too_tall() {
        let err = fails(
            scene(40, 5, Some(STRING_MARK)),
            WriteValue::String("AB\nCD".into()),
        );
        assert!(matches!(err, WriteError::FieldOverflow { .. }));
    }

    // Image

    fn bitmap(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| Rgba([(x * 7) as u8, (y * 11) as u8, 50, 255]))
    }

    #[test]
    fn image_smaller() {
        let mut img = scene(33, 20, None);
        fill(&mut img, Rect::new(30, 25, 40, 30), RED); // old content
        let bmp = bitmap(10, 10);
        assert_eq!(
            write(&mut img, WriteValue::Image(bmp.clone())).unwrap(),
            FieldData::Image
        );
        for y in 0..20 {
            for x in 0..33 {
                let p = *img.get_pixel(13 + x, 13 + y);
                let want = if x < 10 && y < 10 {
                    *bmp.get_pixel(x, y)
                } else {
                    WHITE
                };
                assert_eq!(p, want, "({x},{y})");
            }
        }
    }

    #[test]
    fn image_exact_size() {
        let mut img = scene(33, 20, None);
        let bmp = bitmap(33, 20);
        write(&mut img, WriteValue::Image(bmp.clone())).unwrap();
        for (x, y, p) in bmp.enumerate_pixels() {
            assert_eq!(img.get_pixel(13 + x, 13 + y), p);
        }
    }

    #[test]
    fn image_too_large() {
        let err = fails(scene(33, 20, None), WriteValue::Image(bitmap(34, 20)));
        assert!(matches!(err, WriteError::FieldOverflow { .. }));
    }

    #[test]
    fn image_with_frame_color() {
        let mut bmp = bitmap(10, 10);
        bmp.put_pixel(4, 7, FRAME_COLOR);
        let err = fails(scene(33, 20, None), WriteValue::Image(bmp));
        assert!(matches!(err, WriteError::FrameColor { x: 4, y: 7, .. }));
    }
}
