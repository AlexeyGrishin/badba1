//! Types describing the detected structure of a database image.

/// An axis-aligned rectangle of pixels. All four coordinates are inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub left: u32,
    pub top: u32,
    pub right: u32,
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

/// Thickness of a field's frame on each side, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Borders {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

/// A data box: a rectangular data area surrounded by a frame in the frame color.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Number of the field, starting at 1, in reading order.
    pub id: usize,
    /// The outer edge of the frame.
    pub outer: Rect,
    /// The data area inside the frame.
    pub inner: Rect,
    /// Frame thickness on each side.
    pub borders: Borders,
    /// The field's data type and value, read from its type mark.
    pub data: FieldData,
}

/// A field's data type and, for typed fields, its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldData {
    /// No recognized type mark: the data area is just a bitmap.
    Image,
    /// `filled` of the data area's `total` pixels are the mark color.
    Float {
        color: [u8; 4],
        filled: u64,
        total: u64,
    },
    /// The data area holds `count` separate objects in the mark color.
    Counter { color: [u8; 4], count: u64 },
}

impl FieldData {
    /// The type's name: `"image"`, `"float"` or `"counter"`.
    pub const fn type_name(&self) -> &'static str {
        match self {
            FieldData::Image => "image",
            FieldData::Float { .. } => "float",
            FieldData::Counter { .. } => "counter",
        }
    }

    /// For a float field, the filled share of the data area in percent (0 to 100).
    pub fn percent(&self) -> Option<f64> {
        match *self {
            FieldData::Float { filled, total, .. } if total > 0 => {
                Some(100.0 * filled as f64 / total as f64)
            }
            FieldData::Float { .. } => Some(0.0),
            _ => None,
        }
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    fn type_names() {
        let c = [0, 0, 0, 255];
        assert_eq!(FieldData::Image.type_name(), "image");
        assert_eq!(
            FieldData::Float {
                color: c,
                filled: 0,
                total: 1
            }
            .type_name(),
            "float"
        );
        assert_eq!(
            FieldData::Counter { color: c, count: 0 }.type_name(),
            "counter"
        );
    }

    #[test]
    fn float_percent() {
        let c = [0, 0, 0, 255];
        let pct = |filled, total| {
            FieldData::Float {
                color: c,
                filled,
                total,
            }
            .percent()
            .unwrap()
        };
        assert_eq!(pct(0, 532), 0.0);
        assert!((pct(196, 532) - 36.842).abs() < 0.001);
        assert_eq!(pct(532, 532), 100.0);
        assert_eq!(FieldData::Image.percent(), None);
        assert_eq!(FieldData::Counter { color: c, count: 3 }.percent(), None);
    }

    #[test]
    fn rect_contains_edges() {
        let r = Rect::new(1, 2, 3, 4);
        assert!(r.contains(1, 2) && r.contains(3, 4));
        assert!(!r.contains(0, 2) && !r.contains(3, 5));
    }
}
