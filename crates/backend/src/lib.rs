//! Backend of badba1, an experimental database that keeps its data in an image.
//!
//! Fields are rectangular data areas framed in the color `#BADBA1`. Links are
//! gap-free strokes in the same color that join two or more fields.
//! [`analyze_png`] reads a PNG file and returns its [`Layout`];
//! [`detect`] does the same for an image already in memory.
//!
//! Each field has a data type ([`FieldData`]), set by a 3×3 mark above its
//! top-right corner: `image` (the default), `float`, `counter`, `bool`, `ref` or
//! `string` (text read with the built-in glyph font in `font/`).
//!
//! Fields connected through links form a [`Record`], with its fields in a
//! depth-first order that depends on connections and positions.

mod detect;
mod font;
mod layout;
mod load;
mod records;
mod references;
mod strings;
mod types;

pub use detect::{FRAME_COLOR, detect, is_frame_color};
pub use font::{Font, FontError, GlyphMask, builtin as builtin_font};
pub use layout::{Borders, Field, FieldData, Layout, Link, Record, Rect};
pub use load::{Error, analyze_png};
pub use records::records;
pub use references::{REF_REACH, resolve as resolve_references};
pub use strings::{UNKNOWN_GLYPH, read_text};
pub use types::{BOOL_MARK, COUNTER_MARK, FLOAT_MARK, REF_MARK, STRING_MARK, mark_area};
