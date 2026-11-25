//! Backend of badba1, an experimental database that keeps its data in an image.
//!
//! Fields are rectangular data areas framed in the color `#BADBA1`. Links are
//! gap-free strokes in the same color that join two or more fields.
//! [`analyze_png`] reads a PNG file and returns its [`Layout`];
//! [`detect`] does the same for an image already in memory.
//!
//! Each field has a data type ([`FieldData`]), set by a 3×3 mark above its
//! top-right corner: `image` (the default), `float` or `counter`.

mod detect;
mod layout;
mod load;
mod types;

pub use detect::{FRAME_COLOR, detect, is_frame_color};
pub use layout::{Borders, Field, FieldData, Layout, Link, Rect};
pub use load::{Error, analyze_png};
pub use types::{COUNTER_MARK, FLOAT_MARK, mark_area};
