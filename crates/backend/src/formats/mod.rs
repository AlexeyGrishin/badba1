//! Storage formats: how a database image is read from and written to a file.
//!
//! Every format is one module in this folder implementing [`Format`] (reading
//! a file) and [`Storage`] (keeping a loaded database's pixels and storing
//! writes), and one entry in [`FORMATS`]. Nothing outside this folder knows
//! about a specific format.
//!
//! # Adding a format
//!
//! 1. Write its spec, `openspec/specs/formats/<name>/spec.md`.
//! 2. Add `formats/<name>.rs` with a unit struct implementing [`Format`], and a
//!    [`Storage`] for loaded databases.
//! 3. Add the struct to [`FORMATS`].
//!
//! Loading ([`read_image`], [`load_db`](crate::load_db)), the size limit ([`MAX_PIXELS`]) and the REST server's
//! database listing pick the new format up from the list.

use std::path::Path;

use image::RgbaImage;

use crate::error::Error;
use crate::layout::Rect;

pub mod bmp;
pub mod png;

/// The largest image, in pixels (width × height), that can be loaded:
/// 400,000,000, e.g. 10000×40000.
pub const MAX_PIXELS: u64 = 400_000_000;

/// A file format a database can be stored in.
pub trait Format: Sync {
    /// The format's name, such as `"png"`.
    fn name(&self) -> &'static str;

    /// The file extensions this format owns, lower case without the dot.
    fn extensions(&self) -> &'static [&'static str];

    /// Reads the image's width and height from the file's header, without
    /// decoding its pixels.
    fn dimensions(&self, path: &Path) -> Result<(u32, u32), Error>;

    /// Reads the whole image as RGBA pixels. Called only after
    /// [`dimensions`](Format::dimensions) passed the size check.
    fn read(&self, path: &Path) -> Result<RgbaImage, Error>;

    /// Creates the storage of a loaded database from its decoded image. The
    /// storage may keep the image or drop it and read from the file.
    fn storage(&self, path: &Path, image: RgbaImage) -> Result<Box<dyn Storage>, Error>;

    /// Whether this format owns `path`: its extension is one of
    /// [`extensions`](Format::extensions), in any letter case.
    fn owns(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| self.extensions().iter().any(|x| e.eq_ignore_ascii_case(x)))
    }
}

/// Where a loaded database's pixels are kept, and how writes reach the file.
pub trait Storage: Send + Sync {
    /// The pixels of `rect`, as currently stored.
    fn read_rect(&self, rect: Rect) -> Result<RgbaImage, Error>;

    /// Changes `bitmap` to what the file can hold, such as making every pixel
    /// opaque when the file has no alpha channel. Applied to bitmaps before
    /// they are checked and written, so checks see the stored pixels.
    fn stored(&self, _bitmap: &mut RgbaImage) {}

    /// Replaces the pixels of `rect` with `pixels` (of `rect`'s size) and
    /// saves the change. All or nothing: on error, the stored pixels and the
    /// file are as before (as far as the file can be restored).
    fn write_rect(&mut self, rect: Rect, pixels: &RgbaImage) -> Result<(), Error>;
}

/// The registered formats. The first one is the default for files whose
/// extension no format owns.
pub static FORMATS: &[&dyn Format] = &[&png::Png, &bmp::Bmp];

/// The format of `path`: the registered format owning its extension, or the
/// first registered format (PNG).
pub fn format_for(path: &Path) -> &'static dyn Format {
    FORMATS
        .iter()
        .copied()
        .find(|f| f.owns(path))
        .unwrap_or(FORMATS[0])
}

/// Every extension owned by a registered format.
pub fn extensions() -> impl Iterator<Item = &'static str> {
    FORMATS.iter().flat_map(|f| f.extensions().iter().copied())
}

/// Reads the image at `path` in its format (see [`format_for`]), rejecting
/// images over [`MAX_PIXELS`] from the header, before decoding.
pub fn read_image(path: &Path) -> Result<(&'static dyn Format, RgbaImage), Error> {
    let format = format_for(path);
    let (width, height) = format.dimensions(path)?;
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Error::TooLarge {
            path: path.to_path_buf(),
            width,
            height,
        });
    }
    Ok((format, format.read(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_formats() {
        let names: Vec<_> = FORMATS.iter().map(|f| (f.name(), f.extensions())).collect();
        assert_eq!(names, [("png", &["png"][..]), ("bmp", &["bmp"][..])]);
        assert_eq!(extensions().collect::<Vec<_>>(), ["png", "bmp"]);
    }

    #[test]
    fn format_by_extension_in_any_case() {
        assert_eq!(format_for(Path::new("a.png")).name(), "png");
        assert_eq!(format_for(Path::new("a.bmp")).name(), "bmp");
        assert_eq!(format_for(Path::new("EXAMPLE1.BMP")).name(), "bmp");
        assert_eq!(format_for(Path::new("x/a.Bmp")).name(), "bmp");
    }

    #[test]
    fn unknown_extension_is_png() {
        assert_eq!(format_for(Path::new("example1.img")).name(), "png");
        assert_eq!(format_for(Path::new("noext")).name(), "png");
    }

    #[test]
    fn unknown_extension_reads_a_png() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/example1.png");
        let path = std::env::temp_dir().join(format!("badba1-{}-example1.img", std::process::id()));
        std::fs::copy(src, &path).unwrap();
        let (format, image) = read_image(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(format.name(), "png");
        assert_eq!(image.dimensions(), (539, 348));
    }
}
