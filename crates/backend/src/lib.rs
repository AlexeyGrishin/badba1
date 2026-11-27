//! badba1: an experimental database that keeps its data in an image, so it
//! can be edited with any image editor as well as through this library.
//!
//! # The model
//!
//! - A **field** is a rectangular data area framed in the color `#BADBA1`
//!   ([`FRAME_COLOR`]).
//! - A **link** is a gap-free stroke in the frame color joining two or more
//!   fields. Fields connected through links form a **record**, with its fields
//!   in a depth-first order that depends on connections and positions.
//! - Each field has a **type**, set by a 3×3 mark above its top-right corner:
//!   `image` (no mark), `float`, `counter`, `bool`, `ref` or `string`. Its
//!   value is read from what is drawn in the data area, and is one of a few
//!   generic kinds ([`Value`]).
//! - The image is stored in a file in one of the **storage formats**, PNG or
//!   BMP, chosen by the file extension.
//!
//! # The entry point
//!
//! [`load_db`] opens a database file and returns a [`Database`]. Everything
//! else is a method of that handle:
//!
//! | Operation | Method |
//! |-----------|--------|
//! | load a database | [`load_db`] / [`Database::load`] |
//! | list records | [`Database::records`] |
//! | get a record by id (`"x/y"`) | [`Database::record`] |
//! | get a field (type, value, rectangles) | [`Database::field`] |
//! | get a field's pixels | [`Database::field_image`] |
//! | write a field | [`Database::set_field`] |
//!
//! Errors are reported as [`Error`]. A failed write leaves the database and
//! its file unchanged.
//!
//! ```
//! use backend::{Input, RecordId, Value, load_db};
//!
//! # let dir = std::env::temp_dir().join(format!("badba1-doc-{}", std::process::id()));
//! # std::fs::create_dir_all(&dir)?;
//! # let path = dir.join("example3.png");
//! # std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/example3.png"), &path)?;
//! // `path` is a copy of testdata/example3.png.
//! let mut db = load_db(&path)?;
//! let ids: Vec<String> = db.records().map(|r| r.id().to_string()).collect();
//! assert_eq!(ids, ["35/21", "250/23", "35/68"]);
//!
//! // Field 1 of record 35/21 is a counter holding 4.
//! let record: RecordId = "35/21".parse()?;
//! let field = db.field(record, 1)?;
//! assert_eq!(field.type_name(), "counter");
//! assert_eq!(field.value(), &Value::Int(4));
//! assert_eq!(field.cli_text(), "counter 4");
//!
//! // Write 7 into it: the file is updated, and so is the handle.
//! let stored = db.set_field(record, 1, Input::Int(7))?;
//! assert_eq!(stored.value(), &Value::Int(7));
//! assert_eq!(load_db(&path)?.field(record, 1)?.value(), &Value::Int(7));
//!
//! // Field 0 is an image field: its pixels can be read.
//! assert_eq!(db.field_image(record, 0)?.dimensions(), (33, 20));
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Extending
//!
//! Every field type is one module in [`types`], listed in [`types::TYPES`];
//! every storage format is one module in [`formats`], listed in
//! [`formats::FORMATS`]. See those modules for how to add one. Nothing else
//! in the library, the CLI or the server knows about a specific type or
//! format.
//!
//! # Lower-level functions
//!
//! [`detect`] finds the layout of an image already in memory, and
//! [`write_field`] writes into such an image; both are what [`Database`]
//! uses.

#![warn(missing_docs)]

mod db;
mod detect;
mod draw;
mod error;
mod field_map;
mod font;
pub mod formats;
mod layout;
mod records;
pub mod types;

pub use db::{Database, FieldView, RecordView, load_db};
pub use detect::{FRAME_COLOR, detect, is_frame_color};
pub use draw::{Plan, WriteError, write_field};
pub use error::Error;
pub use font::{Font, FontError, GlyphMask, builtin as builtin_font};
pub use formats::MAX_PIXELS;
pub use layout::{Borders, Field, Layout, Link, Record, RecordId, Rect};
pub use records::records;
pub use types::{FieldType, Input, InputKind, Value};
