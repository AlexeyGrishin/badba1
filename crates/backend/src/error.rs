//! The library's error type.

use std::fmt;
use std::path::PathBuf;

use crate::formats::MAX_PIXELS;
use crate::layout::RecordId;
use crate::draw::WriteError;

/// Why a database operation failed.
///
/// Load errors ([`Error::Io`], [`Error::Decode`], [`Error::Unsupported`],
/// [`Error::TooLarge`]) name the file; lookups name the record and field;
/// write errors carry a [`WriteError`]; [`Error::Save`] means a valid write
/// could not be stored, and the database was left as it was.
#[derive(Debug)]
pub enum Error {
    /// The file could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        source: std::io::Error,
    },
    /// The file is not a valid file of its format.
    Decode {
        /// The file.
        path: PathBuf,
        /// The format's display name, such as `PNG`.
        format: &'static str,
        /// What is wrong with the file.
        reason: String,
    },
    /// The file is a variant of its format that is not supported.
    Unsupported {
        /// The file.
        path: PathBuf,
        /// The format's display name, such as `BMP`.
        format: &'static str,
        /// Which variant it is.
        reason: String,
    },
    /// The image has more than [`MAX_PIXELS`] pixels.
    TooLarge {
        /// The file.
        path: PathBuf,
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
    },
    /// The database has no record with this id.
    NoSuchRecord(RecordId),
    /// The record has no field with this index.
    NoSuchField {
        /// The record.
        record: RecordId,
        /// The field index, counting from 0 in record order.
        index: usize,
    },
    /// The value cannot be written into the field. Nothing was changed.
    Write(WriteError),
    /// A valid write could not be saved to the file. Nothing was changed.
    Save {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        reason: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            Error::Decode {
                path,
                format,
                reason,
            } => write!(f, "cannot decode {} as {format}: {reason}", path.display()),
            Error::Unsupported {
                path,
                format,
                reason,
            } => write!(
                f,
                "cannot load {}: unsupported {format} format: {reason}",
                path.display()
            ),
            Error::TooLarge {
                path,
                width,
                height,
            } => write!(
                f,
                "cannot load {}: image is too large: {width}x{height} ({} pixels), maximum is {MAX_PIXELS} pixels",
                path.display(),
                u64::from(*width) * u64::from(*height)
            ),
            Error::NoSuchRecord(id) => write!(f, "no record {id}"),
            Error::NoSuchField { record, index } => {
                write!(f, "record {record} has no field {index}")
            }
            Error::Write(e) => e.fmt(f),
            Error::Save { reason, .. } => write!(f, "cannot save: {reason}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Write(e) => Some(e),
            _ => None,
        }
    }
}

impl From<WriteError> for Error {
    fn from(e: WriteError) -> Self {
        Error::Write(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages() {
        let path = PathBuf::from("db.png");
        let id = RecordId { x: 35, y: 21 };
        let cases = [
            (
                Error::Decode {
                    path: path.clone(),
                    format: "PNG",
                    reason: "bad".into(),
                },
                "cannot decode db.png as PNG: bad",
            ),
            (
                Error::Unsupported {
                    path: path.clone(),
                    format: "BMP",
                    reason: "RLE compression".into(),
                },
                "cannot load db.png: unsupported BMP format: RLE compression",
            ),
            (
                Error::TooLarge {
                    path: path.clone(),
                    width: 20001,
                    height: 20000,
                },
                "cannot load db.png: image is too large: 20001x20000 (400020000 pixels), maximum is 400000000 pixels",
            ),
            (Error::NoSuchRecord(id), "no record 35/21"),
            (
                Error::NoSuchField {
                    record: id,
                    index: 9,
                },
                "record 35/21 has no field 9",
            ),
            (
                Error::Write(WriteError::NoSuchField(99)),
                "no field F99",
            ),
            (
                Error::Save {
                    path,
                    reason: "disk full".into(),
                },
                "cannot save: disk full",
            ),
        ];
        for (err, text) in cases {
            assert_eq!(err.to_string(), text);
        }
    }
}
