//! The PNG storage format: any standard PNG color type and bit depth is read;
//! a loaded database keeps its pixels in memory, and a write saves the whole
//! image as an 8-bit RGBA PNG through a temporary file and a rename.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use image::{GenericImage, ImageFormat, ImageReader, Limits, RgbaImage};

use super::{Format, MAX_PIXELS, Storage};
use crate::error::Error;
use crate::layout::Rect;

/// The PNG format: files ending in `.png`.
pub struct Png;

fn io_error(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn decode_error(path: &Path, source: image::ImageError) -> Error {
    Error::Decode {
        path: path.to_path_buf(),
        format: "PNG",
        reason: source.to_string(),
    }
}

impl Format for Png {
    fn name(&self) -> &'static str {
        "png"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }

    fn dimensions(&self, path: &Path) -> Result<(u32, u32), Error> {
        let file = File::open(path).map_err(|e| io_error(path, e))?;
        ImageReader::with_format(BufReader::new(file), ImageFormat::Png)
            .into_dimensions()
            .map_err(|e| decode_error(path, e))
    }

    fn read(&self, path: &Path) -> Result<RgbaImage, Error> {
        let bytes = std::fs::read(path).map_err(|e| io_error(path, e))?;
        let mut limits = Limits::default();
        // 8 bytes per pixel covers the widest PNG layout, 16-bit RGBA.
        limits.max_alloc = Some(MAX_PIXELS * 8);
        let mut reader = ImageReader::with_format(Cursor::new(&bytes), ImageFormat::Png);
        reader.limits(limits);
        let image = reader.decode().map_err(|e| decode_error(path, e))?;
        drop(bytes);
        Ok(image.into_rgba8())
    }

    fn storage(&self, path: &Path, image: RgbaImage) -> Result<Box<dyn Storage>, Error> {
        Ok(Box::new(PngStorage {
            path: path.to_path_buf(),
            image,
        }))
    }
}

/// A loaded PNG database: its pixels, held in memory.
struct PngStorage {
    path: PathBuf,
    image: RgbaImage,
}

impl PngStorage {
    /// Saves the image via a temporary file in the same folder.
    fn save(&self) -> Result<(), String> {
        let tmp = self.path.with_extension("png.tmp");
        self.image
            .save_with_format(&tmp, ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

impl Storage for PngStorage {
    fn read_rect(&self, r: Rect) -> Result<RgbaImage, Error> {
        Ok(image::imageops::crop_imm(&self.image, r.left, r.top, r.width(), r.height()).to_image())
    }

    fn write_rect(&mut self, r: Rect, pixels: &RgbaImage) -> Result<(), Error> {
        let before = self.read_rect(r)?;
        self.image
            .copy_from(pixels, r.left, r.top)
            .expect("the pixels fit in the rectangle");
        if let Err(reason) = self.save() {
            self.image
                .copy_from(&before, r.left, r.top)
                .expect("the copy fits where it came from");
            return Err(Error::Save {
                path: self.path.clone(),
                reason,
            });
        }
        Ok(())
    }
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
        let err = super::super::read_image(path).map(|(_, i)| i).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
        assert!(err.to_string().contains("does-not-exist.png"));
    }

    #[test]
    fn text_file_is_decode_error() {
        let path = temp_file("not-a-png.png", b"just some text");
        let err = super::super::read_image(&path).map(|(_, i)| i).unwrap_err();
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
        let err = super::super::read_image(&path).map(|(_, i)| i).unwrap_err();
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
        let err = super::super::read_image(&path).map(|(_, i)| i).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, Error::TooLarge { .. }));
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn at_the_maximum_is_not_too_large() {
        // Passes the size check, then fails decoding the missing pixel data.
        let path = temp_file("max.png", &header_only_png(20000, 20000));
        let err = super::super::read_image(&path).map(|(_, i)| i).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, Error::Decode { .. }), "{err}");
    }

    #[test]
    fn valid_png_reports_size() {
        let mut img = crate::detect::tests::blank(30, 20);
        crate::detect::tests::frame(&mut img, crate::Rect::new(5, 5, 14, 14), (2, 2, 2, 2));
        let path = std::env::temp_dir().join(format!("badba1-{}-valid.png", std::process::id()));
        img.save(&path).unwrap();
        let layout = crate::detect::detect(&super::super::read_image(&path).unwrap().1);
        std::fs::remove_file(&path).ok();
        assert_eq!((layout.width, layout.height), (30, 20));
        assert_eq!(layout.fields.len(), 1);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("badba1-png-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn write_rect_saves_the_file() {
        let dir = temp_dir("save");
        let path = dir.join("db.png");
        RgbaImage::from_pixel(4, 3, image::Rgba([255, 255, 255, 255]))
            .save(&path)
            .unwrap();
        let image = Png.read(&path).unwrap();
        let mut storage = Png.storage(&path, image).unwrap();
        let patch = RgbaImage::from_pixel(2, 1, image::Rgba([1, 2, 3, 255]));
        storage.write_rect(Rect::new(1, 1, 2, 1), &patch).unwrap();
        let saved = Png.read(&path).unwrap();
        let leftovers = std::fs::read_dir(&dir).unwrap().count();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(saved.get_pixel(1, 1).0, [1, 2, 3, 255]);
        assert_eq!(saved.get_pixel(0, 1).0, [255, 255, 255, 255]);
        assert_eq!(leftovers, 1, "no temporary file is left");
    }

    #[test]
    fn failed_save_keeps_memory_and_file() {
        let dir = temp_dir("fail");
        let path = dir.join("db.png");
        RgbaImage::from_pixel(4, 3, image::Rgba([255, 255, 255, 255]))
            .save(&path)
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        let image = Png.read(&path).unwrap();
        let mut storage = Png.storage(&path, image).unwrap();
        // A folder where the temporary file should go makes every save fail.
        std::fs::create_dir(dir.join("db.png.tmp")).unwrap();
        let patch = RgbaImage::from_pixel(2, 1, image::Rgba([1, 2, 3, 255]));
        let err = storage
            .write_rect(Rect::new(1, 1, 2, 1), &patch)
            .unwrap_err();
        let after = std::fs::read(&path).unwrap();
        let pixels = storage.read_rect(Rect::new(0, 0, 3, 2)).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert!(matches!(err, Error::Save { .. }), "{err}");
        assert!(err.to_string().starts_with("cannot save: "), "{err}");
        assert_eq!(after, before);
        assert!(pixels.pixels().all(|p| p.0 == [255, 255, 255, 255]));
    }
}
