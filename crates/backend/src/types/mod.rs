//! Field types: each field's data type, read from a 3×3 mark above its
//! top-right corner, and how values of that type are read, shown and written.
//!
//! Every type is one module in this folder implementing [`FieldType`], and
//! one entry in [`TYPES`]. Nothing outside this folder knows about a specific
//! type: detection, writes, the [`Database`](crate::Database), the CLI and the
//! server all go through the trait and the generic [`Value`] and [`Input`].
//!
//! # Adding a type
//!
//! 1. Write its spec, `openspec/specs/types/<name>/spec.md`.
//! 2. Add `types/<name>.rs` with a unit struct implementing [`FieldType`]: its
//!    name, a mark shape no other type uses, how its value is read from the
//!    data area, its CLI text, and (if it can be written) its input kind and
//!    write plan.
//! 3. Add the struct to [`TYPES`].

use std::fmt;

use image::RgbaImage;

use crate::detect::FRAME_COLOR;
use crate::draw::{Plan, WriteError};
use crate::field_map::FieldMap;
use crate::layout::{Field, Record, Rect};

#[path = "bool.rs"]
mod bool_type;
mod counter;
mod float;
mod image_type;
#[path = "ref.rs"]
mod ref_type;
mod string;

pub use bool_type::Bool;
pub use counter::Counter;
pub use float::Float;
pub use image_type::Image;
pub use ref_type::{REF_REACH, Reference};
pub use string::{Str, UNKNOWN_GLYPH};

/// A type mark: 3 rows of 3 pixels, top to bottom, `true` where the pixel
/// has the mark color.
pub type Mark = [[bool; 3]; 3];

/// A field's value, in one of a fixed set of kinds that does not grow with
/// the number of types. Clients can show and convert every type's value by
/// its kind alone.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// No value; the field is a bitmap of this size (the data area's).
    Bitmap {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// A whole number.
    Int(u64),
    /// A decimal number.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// Text; lines are joined by `\n`.
    Text(String),
    /// A reference to a record, by its number (starting at 1), or null.
    Record(Option<usize>),
}

/// The kind of value a write takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    /// A whole number: [`Input::Int`].
    Int,
    /// A decimal number: [`Input::Number`].
    Number,
    /// A boolean: [`Input::Bool`].
    Bool,
    /// Text: [`Input::Text`].
    Text,
    /// A bitmap: [`Input::Bitmap`].
    Bitmap,
}

/// A value to write into a field. Its kind must be the field type's
/// [`input_kind`](FieldType::input_kind).
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// A whole number.
    Int(u64),
    /// A decimal number.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// Text; `\n` starts a new line.
    Text(String),
    /// A bitmap, placed at the data area's top-left.
    Bitmap(RgbaImage),
}

impl Input {
    /// This input's kind.
    pub fn kind(&self) -> InputKind {
        match self {
            Input::Int(_) => InputKind::Int,
            Input::Number(_) => InputKind::Number,
            Input::Bool(_) => InputKind::Bool,
            Input::Text(_) => InputKind::Text,
            Input::Bitmap(_) => InputKind::Bitmap,
        }
    }
}

/// A field's data area, as seen by a type reading its value: pixels relative
/// to the data area's top-left, and the image's background color (its
/// top-left pixel).
pub struct AreaView<'a> {
    image: &'a RgbaImage,
    rect: Rect,
    background: [u8; 4],
}

impl<'a> AreaView<'a> {
    /// The area `rect` of `image`, with the given background color.
    pub fn new(image: &'a RgbaImage, rect: Rect, background: [u8; 4]) -> Self {
        AreaView {
            image,
            rect,
            background,
        }
    }

    /// All of `bitmap`, which holds just a data area.
    pub fn whole(bitmap: &'a RgbaImage, background: [u8; 4]) -> Self {
        let (w, h) = bitmap.dimensions();
        AreaView::new(bitmap, Rect::new(0, 0, w - 1, h - 1), background)
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.rect.width()
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.rect.height()
    }

    /// The background color.
    pub fn background(&self) -> [u8; 4] {
        self.background
    }

    /// The color of pixel (`x`, `y`), relative to the area's top-left.
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        self.image.get_pixel(self.rect.left + x, self.rect.top + y).0
    }

    /// The colors of all pixels, row by row.
    pub fn pixels(&self) -> impl Iterator<Item = [u8; 4]> + '_ {
        (0..self.height()).flat_map(move |y| (0..self.width()).map(move |x| self.get(x, y)))
    }
}

/// What a type's second pass ([`FieldType::resolve`]) can see: the whole
/// image, the records, and where the fields are.
pub struct ResolveCtx<'a> {
    /// The whole image.
    pub image: &'a RgbaImage,
    /// The records, ordered by id.
    pub records: &'a [Record],
    pub(crate) map: &'a FieldMap,
}

/// A field data type. See the [module docs](self) for how to add one.
pub trait FieldType: Sync {
    /// The type's name, such as `"counter"`.
    fn name(&self) -> &'static str;

    /// The type's mark shape; `None` only for the default type, `image`.
    fn mark(&self) -> Option<Mark>;

    /// The value of a field of this type, read from its data area. `color`
    /// is the mark color.
    fn read(&self, area: &AreaView<'_>, color: [u8; 4]) -> Value;

    /// A second pass over all fields once records are known, for types whose
    /// value depends on the rest of the image. Called once per type; most
    /// types do nothing.
    fn resolve(&self, _ctx: &ResolveCtx<'_>, _fields: &mut [Field]) {}

    /// The kind of input a write takes, or `None` if fields of this type
    /// cannot be written.
    fn input_kind(&self) -> Option<InputKind>;

    /// Validates `input` (already of [`input_kind`](FieldType::input_kind))
    /// for `field` and works out what to draw into its cleared data area.
    fn plan(&self, field: &Field, _input: Input) -> Result<Plan, WriteError> {
        Err(WriteError::NotWritable {
            field: field.id,
            type_name: self.name(),
        })
    }

    /// The value as the CLI shows it, such as `counter 5`.
    fn cli_text(&self, value: &Value) -> String;
}

impl fmt::Debug for dyn FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl PartialEq for dyn FieldType {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}

/// The registered field types. `image` comes first: it is the default for
/// fields without a recognized mark.
pub static TYPES: &[&dyn FieldType] = &[&Image, &Float, &Counter, &Bool, &Reference, &Str];

/// The registered type named `name`.
pub fn by_name(name: &str) -> Option<&'static dyn FieldType> {
    TYPES.iter().copied().find(|t| t.name() == name)
}

/// The name of the registered type whose writes take `kind`.
pub(crate) fn name_for_input(kind: InputKind) -> &'static str {
    TYPES
        .iter()
        .find(|t| t.input_kind() == Some(kind))
        .map_or("unknown", |t| t.name())
}

/// The 3×3 area holding a field's type mark: directly above the top-right
/// corner of the outer rectangle. `None` if there is no room above the field.
pub fn mark_area(outer: Rect) -> Option<Rect> {
    (outer.top >= 3 && outer.right >= 2)
        .then(|| Rect::new(outer.right - 2, outer.top - 3, outer.right, outer.top - 1))
}

/// Reads the type of the field with outer rectangle `outer` from its mark:
/// the registered type whose shape matches, and the mark color; `image`
/// (without a color) if none does.
pub fn recognize(image: &RgbaImage, outer: Rect) -> (&'static dyn FieldType, Option<[u8; 4]>) {
    let default = (TYPES[0], None);
    let Some(area) = mark_area(outer) else {
        return default;
    };
    // Every mark shape holds the top-middle pixel; the top-left may be empty.
    let color = image.get_pixel(area.left + 1, area.top).0;
    if color == FRAME_COLOR.0 {
        return default;
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
    TYPES
        .iter()
        .copied()
        .find(|t| t.mark() == Some(shape))
        .map_or(default, |t| (t, Some(color)))
}

/// Reads a field's type and value from `image` (whose top-left pixel is the
/// background), filling in its `kind`, `color` and `value`.
pub(crate) fn read_field(image: &RgbaImage, field: &mut Field) {
    let (kind, color) = recognize(image, field.outer);
    let background = image.get_pixel(0, 0).0;
    let area = AreaView::new(image, field.inner, background);
    field.kind = kind;
    field.color = color;
    field.value = kind.read(&area, color.unwrap_or(background));
}

#[cfg(test)]
pub(crate) mod testing {
    //! Helpers for the types' tests: a field with a given mark, and writes
    //! into a single field.

    use super::*;
    use crate::detect::detect;
    use crate::detect::tests::{blank, frame};
    use crate::draw::write_field;
    use image::Rgba;

    /// Outer rectangle of the test field: data area (15,15)-(24,24), 10×10.
    pub const OUTER: Rect = Rect::new(10, 10, 29, 29);

    /// Draws `shape` in `color` into the mark area of `outer`.
    pub fn draw_mark(img: &mut RgbaImage, outer: Rect, shape: Mark, color: Rgba<u8>) {
        let area = mark_area(outer).unwrap();
        for (dy, row) in shape.iter().enumerate() {
            for (dx, &on) in row.iter().enumerate() {
                if on {
                    img.put_pixel(area.left + dx as u32, area.top + dy as u32, color);
                }
            }
        }
    }

    /// A 40×40 image with one field at `OUTER` (5px borders) and the mark.
    pub fn field_with_mark(shape: Mark, color: Rgba<u8>) -> RgbaImage {
        let mut img = blank(40, 40);
        frame(&mut img, OUTER, (5, 5, 5, 5));
        draw_mark(&mut img, OUTER, shape, color);
        img
    }

    /// The single field of `img`.
    pub fn field_of(img: &RgbaImage) -> Field {
        let layout = detect(img);
        assert_eq!(layout.fields.len(), 1);
        layout.fields[0].clone()
    }

    /// The type name and value of the single field of `img`.
    pub fn data_of(img: &RgbaImage) -> (&'static str, Value) {
        let f = field_of(img);
        (f.type_name(), f.value)
    }

    /// An image with one field whose data area is `w`×`h` at (13,13), framed
    /// with 3px borders, and the given mark (or none) in red.
    pub fn scene(w: u32, h: u32, mark: Option<Mark>) -> RgbaImage {
        let outer = Rect::new(10, 10, 10 + w + 5, 10 + h + 5);
        let mut img = blank(outer.right + 10, outer.bottom + 10);
        frame(&mut img, outer, (3, 3, 3, 3));
        if let Some(shape) = mark {
            draw_mark(&mut img, outer, shape, RED);
        }
        img
    }

    pub const RED: Rgba<u8> = Rgba([237, 28, 36, 255]);
    pub const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);

    /// Writes into the scene's single field (F1).
    pub fn write(img: &mut RgbaImage, input: Input) -> Result<Value, WriteError> {
        let layout = detect(img);
        assert_eq!(layout.fields.len(), 1);
        write_field(img, &layout, 1, input)
    }

    /// Asserts that `input` fails on the scene, leaving it unchanged.
    pub fn fails(mut img: RgbaImage, input: Input) -> WriteError {
        let before = img.clone();
        let err = write(&mut img, input).unwrap_err();
        assert_eq!(img, before, "image changed by a failed write");
        err
    }

    /// The non-white pixels of the scene's `w`×`h` data area.
    pub fn data_pixels(img: &RgbaImage, w: u32, h: u32) -> Vec<(u32, u32)> {
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
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use crate::detect::tests::{blank, frame};
    use image::Rgba;

    const GREEN: Rgba<u8> = Rgba([34, 177, 76, 255]);
    const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);

    #[test]
    fn registered_types() {
        let names: Vec<_> = TYPES.iter().map(|t| t.name()).collect();
        assert_eq!(names, ["image", "float", "counter", "bool", "ref", "string"]);
        assert!(TYPES[0].mark().is_none());
        assert!(TYPES[1..].iter().all(|t| t.mark().is_some()));
        assert_eq!(by_name("counter").unwrap().name(), "counter");
        assert!(by_name("nope").is_none());
    }

    #[test]
    fn mark_shapes_are_unique_and_hold_the_top_middle_pixel() {
        let marks: Vec<Mark> = TYPES.iter().filter_map(|t| t.mark()).collect();
        for (i, a) in marks.iter().enumerate() {
            assert!(a[0][1], "shape {a:?} lacks the top-middle pixel");
            for b in &marks[i + 1..] {
                assert_ne!(a, b, "two types share a mark");
            }
        }
    }

    #[test]
    fn input_kinds_name_one_type_each() {
        let kinds = [
            (InputKind::Int, "counter"),
            (InputKind::Number, "float"),
            (InputKind::Bool, "bool"),
            (InputKind::Text, "string"),
            (InputKind::Bitmap, "image"),
        ];
        for (kind, name) in kinds {
            assert_eq!(name_for_input(kind), name);
        }
    }

    // Mark location and recognition

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
        assert_eq!(data_of(&img).0, "image");
    }

    #[test]
    fn shape_selects_the_registered_type() {
        let counter = by_name("counter").unwrap().mark().unwrap();
        let img = field_with_mark(counter, RED);
        let f = field_of(&img);
        assert_eq!(f.type_name(), "counter");
        assert_eq!(f.color, Some(RED.0));
    }

    #[test]
    fn mark_color_from_the_top_middle_pixel() {
        // A float shape whose other pixels are black: only red pixels count.
        let float = by_name("float").unwrap().mark().unwrap();
        let mut img = field_with_mark(float, RED);
        let area = mark_area(OUTER).unwrap();
        img.put_pixel(area.right, area.top + 1, BLACK);
        img.put_pixel(area.left + 1, area.bottom, BLACK);
        let f = field_of(&img);
        assert_eq!((f.type_name(), f.color), ("float", Some(RED.0)));
    }

    #[test]
    fn no_mark() {
        let img = field_with_mark([[false; 3]; 3], GREEN);
        let f = field_of(&img);
        assert_eq!((f.type_name(), f.color), ("image", None));
    }

    #[test]
    fn unrecognized_shape() {
        let x = [
            [true, false, true],
            [false, true, false],
            [true, false, true],
        ];
        assert_eq!(data_of(&field_with_mark(x, GREEN)).0, "image");
    }

    #[test]
    fn frame_colored_mark() {
        // The mark sits right above the frame, so in the frame color it would
        // also make the frame look thicker; recognize the type directly too.
        let float = by_name("float").unwrap().mark().unwrap();
        let img = field_with_mark(float, FRAME_COLOR);
        assert_eq!(recognize(&img, OUTER).0.name(), "image");
        assert_eq!(data_of(&img).0, "image");
    }

    #[test]
    fn area_view_reads_relative_pixels() {
        let mut img = blank(10, 10);
        img.put_pixel(3, 4, BLACK);
        let view = AreaView::new(&img, Rect::new(2, 2, 6, 7), [255; 4]);
        assert_eq!((view.width(), view.height()), (5, 6));
        assert_eq!(view.get(1, 2), BLACK.0);
        assert_eq!(view.pixels().filter(|p| *p == BLACK.0).count(), 1);
    }
}
