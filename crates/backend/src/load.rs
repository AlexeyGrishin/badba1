//! Loading database images from PNG files.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::detect::detect;
use crate::layout::Layout;

/// Why an image could not be analyzed.
#[derive(Debug)]
pub enum Error {
    /// The file could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The file was read but is not a valid PNG.
    Decode {
        path: PathBuf,
        source: image::ImageError,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            Error::Decode { path, source } => {
                write!(f, "cannot decode {} as PNG: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Decode { source, .. } => Some(source),
        }
    }
}

/// Reads the PNG file at `path` and detects its fields and links.
pub fn analyze_png(path: &Path) -> Result<Layout, Error> {
    let bytes = std::fs::read(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|source| Error::Decode {
            path: path.to_path_buf(),
            source,
        })?
        .to_rgba8();
    Ok(detect(&image))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("badba1-{}-{name}", std::process::id()));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn missing_file_is_io_error_naming_path() {
        let path = Path::new("does-not-exist.png");
        let err = analyze_png(path).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
        assert!(err.to_string().contains("does-not-exist.png"));
    }

    #[test]
    fn text_file_is_decode_error() {
        let path = temp_file("not-a-png.png", b"just some text");
        let err = analyze_png(&path).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, Error::Decode { .. }));
        assert!(err.to_string().contains("as PNG"));
    }

    #[test]
    fn valid_png_reports_size() {
        let mut img = crate::detect::tests::blank(30, 20);
        crate::detect::tests::frame(&mut img, crate::Rect::new(5, 5, 14, 14), (2, 2, 2, 2));
        let path = std::env::temp_dir().join(format!("badba1-{}-valid.png", std::process::id()));
        img.save(&path).unwrap();
        let layout = analyze_png(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!((layout.width, layout.height), (30, 20));
        assert_eq!(layout.fields.len(), 1);
    }
}
