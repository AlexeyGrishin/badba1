//! Per-pixel facts about where the fields of an image are, shared by link
//! detection and by types that trace lines between fields (`ref`).

use crate::layout::Field;

/// Which pixels are inside a field, and which field's frame each pixel
/// belongs to. Uses 5 bytes per pixel.
pub(crate) struct FieldMap {
    width: u32,
    height: u32,
    /// Whether the pixel is inside some field's outer rectangle.
    inside: Vec<bool>,
    /// The id of the field whose frame the pixel belongs to, or 0 for none.
    /// Field ids start at 1.
    frame_owner: Vec<u32>,
}

impl FieldMap {
    /// Builds the map for `fields`, which must already be numbered.
    pub(crate) fn new(width: u32, height: u32, fields: &[Field]) -> Self {
        let n = width as usize * height as usize;
        let mut map = FieldMap {
            width,
            height,
            inside: vec![false; n],
            frame_owner: vec![0; n],
        };
        for f in fields {
            let id = u32::try_from(f.id).expect("field id fits in u32");
            debug_assert!(id > 0, "fields must be numbered");
            for y in f.outer.top..=f.outer.bottom {
                for x in f.outer.left..=f.outer.right {
                    let i = map.index(x, y);
                    map.inside[i] = true;
                    if !f.inner.contains(x, y) {
                        map.frame_owner[i] = id;
                    }
                }
            }
        }
        map
    }

    fn index(&self, x: u32, y: u32) -> usize {
        y as usize * self.width as usize + x as usize
    }

    /// Image width in pixels.
    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    /// Whether (`x`, `y`) is inside some field's outer rectangle.
    pub(crate) fn is_inside(&self, x: u32, y: u32) -> bool {
        self.inside[self.index(x, y)]
    }

    /// The id of the field whose frame holds (`x`, `y`), if any.
    pub(crate) fn frame_owner(&self, x: u32, y: u32) -> Option<usize> {
        match self.frame_owner[self.index(x, y)] {
            0 => None,
            id => Some(id as usize),
        }
    }
}
