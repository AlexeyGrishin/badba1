//! Backend of badba1, an experimental database that keeps its data in an image.
//!
//! Fields are rectangular data areas framed in the color `#BADBA1`. Links are
//! gap-free strokes in the same color that join two or more fields.
//! [`analyze_png`] reads a PNG file and returns its [`Layout`];
//! [`detect`] does the same for an image already in memory.

mod detect;
mod layout;
mod load;

pub use detect::{FRAME_COLOR, detect, is_frame_color};
pub use layout::{Borders, Field, Layout, Link, Rect};
pub use load::{Error, analyze_png};
