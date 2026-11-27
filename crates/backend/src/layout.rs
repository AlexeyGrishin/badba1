//! Types describing the detected structure of a database image.

use crate::types::{FieldType, Value};

/// An axis-aligned rectangle of pixels. All four coordinates are inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    /// Leftmost column.
    pub left: u32,
    /// Topmost row.
    pub top: u32,
    /// Rightmost column.
    pub right: u32,
    /// Bottom row.
    pub bottom: u32,
}

impl Rect {
    /// Creates a rectangle from inclusive edge coordinates.
    pub const fn new(left: u32, top: u32, right: u32, bottom: u32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Width in pixels.
    pub const fn width(&self) -> u32 {
        self.right - self.left + 1
    }

    /// Height in pixels.
    pub const fn height(&self) -> u32 {
        self.bottom - self.top + 1
    }

    /// Whether the pixel at (`x`, `y`) lies inside the rectangle.
    pub const fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }
}

/// The id of a record: the top-left corner (`x`, `y`) of its first field's
/// frame. Shown and parsed as `"x/y"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecordId {
    /// Column of the first field's frame's left edge.
    pub x: u32,
    /// Row of the first field's frame's top edge.
    pub y: u32,
}

impl std::fmt::Display for RecordId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.x, self.y)
    }
}

impl std::str::FromStr for RecordId {
    type Err = String;

    /// Parses `"x/y"`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || format!("{s:?} is not a record id of the form x/y");
        let (x, y) = s.split_once('/').ok_or_else(bad)?;
        Ok(RecordId {
            x: x.parse().map_err(|_| bad())?,
            y: y.parse().map_err(|_| bad())?,
        })
    }
}

/// Thickness of a field's frame on each side, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Borders {
    /// Rows above the data area.
    pub top: u32,
    /// Columns right of the data area.
    pub right: u32,
    /// Rows below the data area.
    pub bottom: u32,
    /// Columns left of the data area.
    pub left: u32,
}

/// A data box: a rectangular data area surrounded by a frame in the frame color.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Number of the field, starting at 1, in reading order.
    pub id: usize,
    /// The outer edge of the frame.
    pub outer: Rect,
    /// The data area inside the frame.
    pub inner: Rect,
    /// Frame thickness on each side.
    pub borders: Borders,
    /// The field's data type, read from its type mark.
    pub kind: &'static dyn FieldType,
    /// The mark color; `None` for `image` fields, which have no mark.
    pub color: Option<[u8; 4]>,
    /// The field's value, read from its data area.
    pub value: Value,
}

impl Field {
    /// The name of the field's type, such as `"counter"`.
    pub fn type_name(&self) -> &'static str {
        self.kind.name()
    }

    /// The field's type and value as the CLI shows them, such as `counter 5`.
    pub fn cli_text(&self) -> String {
        self.kind.cli_text(&self.value)
    }

    /// The mark color of a typed field.
    ///
    /// # Panics
    ///
    /// If the field has no mark (an `image` field).
    pub fn mark_color(&self) -> [u8; 4] {
        self.color.expect("typed field has a mark color")
    }
}

/// A gap-free stroke in the frame color that connects two or more fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Number of the link, starting at 1.
    pub id: usize,
    /// Ids of the connected fields, ascending and without duplicates.
    pub fields: Vec<usize>,
}

/// Everything detected in one image.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Fields, ordered by id.
    pub fields: Vec<Field>,
    /// Links, ordered by id.
    pub links: Vec<Link>,
    /// Records (groups of linked fields), ordered by id.
    pub records: Vec<Record>,
}

/// A set of fields connected through links.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Number of the record, starting at 1, in order of its first field.
    pub id: usize,
    /// Ids of the record's fields, in record order.
    pub fields: Vec<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_size_is_inclusive() {
        let r = Rect::new(101, 22, 137, 47);
        assert_eq!(r.width(), 37);
        assert_eq!(r.height(), 26);
        assert_eq!(Rect::new(5, 5, 5, 5).width(), 1);
    }

    #[test]
    fn rect_contains_edges() {
        let r = Rect::new(1, 2, 3, 4);
        assert!(r.contains(1, 2) && r.contains(3, 4));
        assert!(!r.contains(0, 2) && !r.contains(3, 5));
    }
}
