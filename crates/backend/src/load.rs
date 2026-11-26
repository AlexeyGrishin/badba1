//! Loading database images from PNG files.

use std::fmt;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{ImageFormat, ImageReader, Limits, RgbaImage};

use crate::detect::detect;
use crate::layout::Layout;

/// The largest image, in pixels (width × height), that can be loaded:
/// 400,000,000, e.g. 10000×40000.
pub const MAX_PIXELS: u64 = 400_000_000;

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
    /// The PNG has more than [`MAX_PIXELS`] pixels.
    TooLarge {
        path: PathBuf,
        width: u32,
        height: u32,
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
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Decode { source, .. } => Some(source),
            Error::TooLarge { .. } => None,
        }
    }
}

/// Reads the PNG file at `path` as an RGBA image.
///
/// The size is checked from the PNG header first: an image of more than
/// [`MAX_PIXELS`] pixels is rejected before its pixels are decoded.
pub fn load_png(path: &Path) -> Result<RgbaImage, Error> {
    let bytes = std::fs::read(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let decode_err = |source| Error::Decode {
        path: path.to_path_buf(),
        source,
    };
    let (width, height) = ImageReader::with_format(Cursor::new(&bytes), ImageFormat::Png)
        .into_dimensions()
        .map_err(decode_err)?;
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Error::TooLarge {
            path: path.to_path_buf(),
            width,
            height,
        });
    }
    let mut limits = Limits::default();
    // 8 bytes per pixel covers the widest PNG layout, 16-bit RGBA.
    limits.max_alloc = Some(MAX_PIXELS * 8);
    let mut reader = ImageReader::with_format(Cursor::new(&bytes), ImageFormat::Png);
    reader.limits(limits);
    let image = reader.decode().map_err(decode_err)?;
    drop(bytes);
    Ok(image.into_rgba8())
}

/// Reads the PNG file at `path` and detects its fields and links.
pub fn analyze_png(path: &Path) -> Result<Layout, Error> {
    Ok(detect(&load_png(path)?))
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

    /// A PNG with only a signature, an IHDR chunk declaring the given size and
    /// an empty IDAT chunk: enough to read the header, but no pixel data.
    pub(crate) fn header_only_png(width: u32, height: u32) -> Vec<u8> {
        let mut ihdr = b"IHDR".to_vec();
        ihdr.extend(width.to_be_bytes());
        ihdr.extend(height.to_be_bytes());
        ihdr.extend([8, 6, 0, 0, 0]); // 8-bit RGBA, no interlace
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(13u32.to_be_bytes());
        png.extend(&ihdr);
        png.extend(crc32(&ihdr).to_be_bytes());
        png.extend(0u32.to_be_bytes());
        png.extend(b"IDAT");
        png.extend(crc32(b"IDAT").to_be_bytes());
        png
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn over_the_maximum_is_too_large() {
        let path = temp_file("huge.png", &header_only_png(20001, 20000));
        let err = load_png(&path).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(
            err,
            Error::TooLarge {
                width: 20001,
                height: 20000,
                ..
            }
        ));
        let msg = err.to_string();
        assert!(msg.contains("too large"), "{msg}");
        assert!(msg.contains("20001x20000"), "{msg}");
        assert!(msg.contains("400000000"), "{msg}");
        assert!(msg.contains("huge.png"), "{msg}");
    }

    #[test]
    fn huge_header_is_rejected_without_decoding() {
        let path = temp_file("giant.png", &header_only_png(100_000, 100_000));
        let start = std::time::Instant::now();
        let err = analyze_png(&path).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, Error::TooLarge { .. }));
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn at_the_maximum_is_not_too_large() {
        // Passes the size check, then fails decoding the missing pixel data.
        let path = temp_file("max.png", &header_only_png(20000, 20000));
        let err = load_png(&path).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, Error::Decode { .. }), "{err}");
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
