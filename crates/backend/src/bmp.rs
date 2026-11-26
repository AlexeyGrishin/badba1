//! Uncompressed BMP files: header parsing, whole-image reads, and reads and
//! in-place writes of single rectangles.
//!
//! Supported: 24-bit pixels (`BI_RGB`), and 32-bit pixels as `BI_RGB` or as
//! `BI_BITFIELDS` with the standard masks (R `00FF0000`, G `0000FF00`,
//! B `000000FF`, A `FF000000` or none); rows bottom-up or top-down; info
//! headers of 40, 52, 56, 108 or 124 bytes. Pixels count as fully opaque
//! unless the file has an alpha mask.
//!
//! Every pixel row has a fixed size at a fixed offset, so any rectangle can be
//! read or overwritten without touching the rest of the file.

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

use image::RgbaImage;

use crate::layout::Rect;
use crate::load::{Error, MAX_PIXELS};

/// The pixel layout of a supported BMP file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BmpInfo {
    pub width: u32,
    pub height: u32,
    /// Whether the first stored row is the bottom one (the usual order).
    pub bottom_up: bool,
    /// 3 or 4.
    pub bytes_per_pixel: u32,
    /// Whether the 4th byte of a 32-bit pixel is alpha (otherwise it is
    /// ignored and pixels are opaque).
    pub has_alpha: bool,
    /// File offset of the first stored row.
    pub data_offset: u64,
    /// Bytes per stored row, including padding to a multiple of 4.
    pub stride: u64,
}

/// Why a header was refused.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HeaderError {
    /// Not a BMP file at all, or a broken one.
    NotBmp(String),
    /// A BMP variant this module does not handle.
    Unsupported(String),
}

const FILE_HEADER: usize = 14;
/// Enough bytes for the largest info header plus masks after a short one.
const MAX_HEADER: usize = FILE_HEADER + 124;

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}

impl BmpInfo {
    /// Parses the start of a BMP file (up to [`MAX_HEADER`] bytes, fewer if
    /// the file is shorter).
    pub(crate) fn parse(b: &[u8]) -> Result<BmpInfo, HeaderError> {
        let not_bmp = |m: &str| Err(HeaderError::NotBmp(m.to_string()));
        let unsupported = |m: String| Err(HeaderError::Unsupported(m));
        if b.len() < 2 || &b[0..2] != b"BM" {
            return not_bmp("missing BM signature");
        }
        if b.len() < FILE_HEADER + 4 {
            return not_bmp("truncated header");
        }
        let data_offset = u64::from(u32_at(b, 10));
        let header_size = u32_at(b, 14) as usize;
        match header_size {
            40 | 52 | 56 | 108 | 124 => {}
            12 => return unsupported("OS/2 BITMAPCOREHEADER".into()),
            n => return not_bmp(&format!("invalid info header size {n}")),
        }
        if b.len() < FILE_HEADER + header_size {
            return not_bmp("truncated header");
        }
        let width = u32_at(b, 18) as i32;
        let height = u32_at(b, 22) as i32;
        let planes = u16_at(b, 26);
        let bpp = u16_at(b, 28);
        let compression = u32_at(b, 30);
        if width <= 0 || height == 0 || height == i32::MIN {
            return not_bmp("invalid image size");
        }
        if planes != 1 {
            return not_bmp("invalid plane count");
        }
        match compression {
            0 | 3 => {}
            1 | 2 => return unsupported("RLE compression".into()),
            4 => return unsupported("JPEG compression".into()),
            5 => return unsupported("PNG compression".into()),
            n => return unsupported(format!("compression type {n}")),
        }
        if bpp != 24 && bpp != 32 {
            let kind = if bpp <= 8 { " (palette)" } else { "" };
            return unsupported(format!("{bpp}-bit pixels{kind}"));
        }
        let mut has_alpha = false;
        if compression == 3 {
            if bpp != 32 {
                return unsupported(format!("bit fields with {bpp}-bit pixels"));
            }
            // The masks follow a 40-byte header, or sit inside larger ones.
            if b.len() < FILE_HEADER + 40 + 12 {
                return not_bmp("truncated bit masks");
            }
            let (r, g, bl) = (u32_at(b, 54), u32_at(b, 58), u32_at(b, 62));
            let a = if header_size >= 56 { u32_at(b, 66) } else { 0 };
            if (r, g, bl) != (0x00FF_0000, 0x0000_FF00, 0x0000_00FF) || (a != 0 && a != 0xFF00_0000)
            {
                return unsupported(format!(
                    "bit masks R {r:08X} G {g:08X} B {bl:08X} A {a:08X}"
                ));
            }
            has_alpha = a != 0;
        }
        let width = width as u32;
        let stride = (u64::from(width) * u64::from(bpp)).div_ceil(32) * 4;
        Ok(BmpInfo {
            width,
            height: height.unsigned_abs(),
            bottom_up: height > 0,
            bytes_per_pixel: u32::from(bpp) / 8,
            has_alpha,
            data_offset,
            stride,
        })
    }

    /// File offset of image row `y` (0 = top).
    pub fn row_offset(&self, y: u32) -> u64 {
        let stored = if self.bottom_up {
            self.height - 1 - y
        } else {
            y
        };
        self.data_offset + self.stride * u64::from(stored)
    }

    /// File offset just past the pixel data.
    fn data_end(&self) -> u64 {
        self.data_offset + self.stride * u64::from(self.height)
    }

    /// Converts stored pixel bytes to RGBA.
    fn decode_pixels(&self, src: &[u8], dst: &mut [u8]) {
        let bpp = self.bytes_per_pixel as usize;
        for (s, d) in src.chunks_exact(bpp).zip(dst.as_chunks_mut::<4>().0) {
            d[0] = s[2];
            d[1] = s[1];
            d[2] = s[0];
            d[3] = if self.has_alpha { s[3] } else { 255 };
        }
    }

    /// Converts RGBA pixels to stored bytes. Without an alpha channel the
    /// alpha is dropped (24-bit) or stored as 255 (32-bit).
    fn encode_pixels(&self, src: &[u8], dst: &mut [u8]) {
        let bpp = self.bytes_per_pixel as usize;
        for (s, d) in src.as_chunks::<4>().0.iter().zip(dst.chunks_exact_mut(bpp)) {
            d[0] = s[2];
            d[1] = s[1];
            d[2] = s[0];
            if bpp == 4 {
                d[3] = if self.has_alpha { s[3] } else { 255 };
            }
        }
    }
}

fn io_error(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Reads and checks the header of the BMP file at `path`: the format must be
/// supported, the size at most [`MAX_PIXELS`], and the file long enough to
/// hold all pixel rows.
pub fn read_info(path: &Path) -> Result<BmpInfo, Error> {
    let mut file = File::open(path).map_err(|e| io_error(path, e))?;
    let mut head = Vec::with_capacity(MAX_HEADER);
    (&mut file)
        .take(MAX_HEADER as u64)
        .read_to_end(&mut head)
        .map_err(|e| io_error(path, e))?;
    let info = BmpInfo::parse(&head).map_err(|e| match e {
        HeaderError::NotBmp(reason) => Error::NotBmp {
            path: path.to_path_buf(),
            reason,
        },
        HeaderError::Unsupported(reason) => Error::UnsupportedBmp {
            path: path.to_path_buf(),
            reason,
        },
    })?;
    if u64::from(info.width) * u64::from(info.height) > MAX_PIXELS {
        return Err(Error::TooLarge {
            path: path.to_path_buf(),
            width: info.width,
            height: info.height,
        });
    }
    let len = file.metadata().map_err(|e| io_error(path, e))?.len();
    if len < info.data_end() {
        return Err(Error::NotBmp {
            path: path.to_path_buf(),
            reason: "pixel data is truncated".into(),
        });
    }
    Ok(info)
}

/// Reads the whole BMP file at `path` as an RGBA image.
pub fn load_bmp(path: &Path) -> Result<RgbaImage, Error> {
    let info = read_info(path)?;
    let file = File::open(path).map_err(|e| io_error(path, e))?;
    let mut reader = BufReader::with_capacity(1 << 20, file);
    reader
        .seek(SeekFrom::Start(info.data_offset))
        .map_err(|e| io_error(path, e))?;
    let mut image = RgbaImage::new(info.width, info.height);
    let row_bytes = info.width as usize * 4;
    let raw: &mut [u8] = &mut image;
    let mut row = vec![0u8; info.stride as usize];
    for stored in 0..info.height {
        reader.read_exact(&mut row).map_err(|e| io_error(path, e))?;
        let y = if info.bottom_up {
            info.height - 1 - stored
        } else {
            stored
        } as usize;
        info.decode_pixels(&row, &mut raw[y * row_bytes..(y + 1) * row_bytes]);
    }
    Ok(image)
}

fn check_rect(info: &BmpInfo, rect: Rect) -> io::Result<()> {
    if rect.left > rect.right
        || rect.top > rect.bottom
        || rect.right >= info.width
        || rect.bottom >= info.height
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("rectangle {rect:?} is outside the image"),
        ));
    }
    Ok(())
}

/// Reads rectangle `rect` of the BMP in `file`, described by `info`, reading
/// only its rows' bytes.
pub fn read_rect(file: &mut File, info: &BmpInfo, rect: Rect) -> io::Result<RgbaImage> {
    check_rect(info, rect)?;
    let bpp = u64::from(info.bytes_per_pixel);
    let mut out = RgbaImage::new(rect.width(), rect.height());
    let row_bytes = rect.width() as usize * 4;
    let raw: &mut [u8] = &mut out;
    let mut buf = vec![0u8; (u64::from(rect.width()) * bpp) as usize];
    for (i, y) in (rect.top..=rect.bottom).enumerate() {
        file.seek(SeekFrom::Start(
            info.row_offset(y) + u64::from(rect.left) * bpp,
        ))?;
        file.read_exact(&mut buf)?;
        info.decode_pixels(&buf, &mut raw[i * row_bytes..(i + 1) * row_bytes]);
    }
    Ok(out)
}

/// Overwrites the pixels of the BMP in `file` at (`x`, `y`) with `image`,
/// writing only those bytes. The caller decides when to sync.
pub fn write_rect(
    file: &mut File,
    info: &BmpInfo,
    x: u32,
    y: u32,
    image: &RgbaImage,
) -> io::Result<()> {
    if image.width() == 0 || image.height() == 0 {
        return Ok(());
    }
    check_rect(
        info,
        Rect::new(x, y, x + image.width() - 1, y + image.height() - 1),
    )?;
    let bpp = u64::from(info.bytes_per_pixel);
    let row_bytes = image.width() as usize * 4;
    let raw: &[u8] = image;
    let mut buf = vec![0u8; (u64::from(image.width()) * bpp) as usize];
    for i in 0..image.height() {
        let start = i as usize * row_bytes;
        info.encode_pixels(&raw[start..start + row_bytes], &mut buf);
        file.seek(SeekFrom::Start(info.row_offset(y + i) + u64::from(x) * bpp))?;
        file.write_all(&buf)?;
    }
    Ok(())
}

/// Encodes `image` as a BMP: 24-bit `BI_RGB` (alpha dropped) or 32-bit
/// `BI_BITFIELDS` with an alpha mask, rows bottom-up or top-down.
pub fn encode_bmp(image: &RgbaImage, bits_per_pixel: u16, top_down: bool) -> Vec<u8> {
    assert!(bits_per_pixel == 24 || bits_per_pixel == 32);
    let (w, h) = image.dimensions();
    let header_size: u32 = if bits_per_pixel == 32 { 56 } else { 40 };
    let data_offset = FILE_HEADER as u32 + header_size;
    let info = BmpInfo {
        width: w,
        height: h,
        bottom_up: !top_down,
        bytes_per_pixel: u32::from(bits_per_pixel) / 8,
        has_alpha: bits_per_pixel == 32,
        data_offset: u64::from(data_offset),
        stride: (u64::from(w) * u64::from(bits_per_pixel)).div_ceil(32) * 4,
    };
    let file_size = info.data_end();
    let mut out = Vec::with_capacity(file_size as usize);
    out.extend(b"BM");
    out.extend((file_size.min(u64::from(u32::MAX)) as u32).to_le_bytes());
    out.extend([0u8; 4]);
    out.extend(data_offset.to_le_bytes());
    out.extend(header_size.to_le_bytes());
    out.extend((w as i32).to_le_bytes());
    let stored_h = if top_down { -(h as i32) } else { h as i32 };
    out.extend(stored_h.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(bits_per_pixel.to_le_bytes());
    let compression: u32 = if bits_per_pixel == 32 { 3 } else { 0 };
    out.extend(compression.to_le_bytes());
    out.extend(((info.stride * u64::from(h)) as u32).to_le_bytes());
    out.extend(2835u32.to_le_bytes()); // 72 DPI
    out.extend(2835u32.to_le_bytes());
    out.extend([0u8; 8]); // palette counts
    if bits_per_pixel == 32 {
        for mask in [0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF, 0xFF00_0000] {
            out.extend(mask.to_le_bytes());
        }
    }
    let raw: &[u8] = image;
    let row_bytes = w as usize * 4;
    let mut row = vec![0u8; info.stride as usize];
    for stored in 0..h {
        let y = if top_down { stored } else { h - 1 - stored } as usize;
        info.encode_pixels(&raw[y * row_bytes..(y + 1) * row_bytes], &mut row);
        out.extend(&row);
    }
    out
}

/// Saves `image` to `path` as a 24-bit bottom-up BMP (the format MS Paint
/// writes). Alpha is dropped.
pub fn save_bmp(path: &Path, image: &RgbaImage) -> io::Result<()> {
    let mut file = io::BufWriter::new(File::create(path)?);
    file.write_all(&encode_bmp(image, 24, false))?;
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("badba1-bmp-{}-{name}", std::process::id()))
    }

    /// An image whose every pixel differs, with alpha varying too.
    fn sample(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            Rgba([
                (x * 7) as u8,
                (y * 11) as u8,
                (x + y) as u8,
                (200 + x % 50) as u8,
            ])
        })
    }

    fn opaque(mut img: RgbaImage) -> RgbaImage {
        for p in img.pixels_mut() {
            p.0[3] = 255;
        }
        img
    }

    /// A 40-byte-header BMP with the given fields and no pixel data.
    fn header(width: i32, height: i32, bpp: u16, compression: u32) -> Vec<u8> {
        let mut b = b"BM".to_vec();
        b.extend(0u32.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        b.extend(54u32.to_le_bytes());
        b.extend(40u32.to_le_bytes());
        b.extend(width.to_le_bytes());
        b.extend(height.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(bpp.to_le_bytes());
        b.extend(compression.to_le_bytes());
        b.extend([0u8; 20]);
        b
    }

    #[test]
    fn parses_bottom_up_and_top_down() {
        let up = BmpInfo::parse(&header(5, 3, 24, 0)).unwrap();
        assert!(up.bottom_up);
        assert_eq!((up.width, up.height, up.stride), (5, 3, 16));
        assert_eq!(up.row_offset(0), 54 + 32);
        assert_eq!(up.row_offset(2), 54);
        let down = BmpInfo::parse(&header(5, -3, 24, 0)).unwrap();
        assert!(!down.bottom_up);
        assert_eq!(down.height, 3);
        assert_eq!(down.row_offset(0), 54);
    }

    #[test]
    fn parses_32_bit() {
        let info = BmpInfo::parse(&header(3, 2, 32, 0)).unwrap();
        assert_eq!(
            (info.bytes_per_pixel, info.has_alpha, info.stride),
            (4, false, 12)
        );
    }

    #[test]
    fn rejects_unsupported_variants() {
        let unsupported = |b: Vec<u8>| match BmpInfo::parse(&b) {
            Err(HeaderError::Unsupported(reason)) => reason,
            other => panic!("expected unsupported, got {other:?}"),
        };
        assert!(unsupported(header(4, 4, 8, 1)).contains("RLE"));
        assert!(unsupported(header(4, 4, 8, 0)).contains("palette"));
        assert!(unsupported(header(4, 4, 16, 0)).contains("16-bit"));
        let mut odd_masks = header(4, 4, 32, 3);
        odd_masks.extend([0u8; 12]);
        assert!(unsupported(odd_masks).contains("bit masks"));
    }

    #[test]
    fn rejects_non_bmp() {
        assert!(matches!(
            BmpInfo::parse(b"just some text"),
            Err(HeaderError::NotBmp(_))
        ));
    }

    #[test]
    fn too_large_header_is_rejected() {
        let path = temp_path("huge.bmp");
        std::fs::write(&path, header(20001, 20000, 24, 0)).unwrap();
        let err = load_bmp(&path).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(
            matches!(
                err,
                Error::TooLarge {
                    width: 20001,
                    height: 20000,
                    ..
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn truncated_pixels_are_rejected() {
        let path = temp_path("short.bmp");
        let mut bytes = encode_bmp(&sample(5, 4), 24, false);
        bytes.truncate(bytes.len() - 1);
        std::fs::write(&path, bytes).unwrap();
        let err = load_bmp(&path).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(err.to_string().contains("as BMP"), "{err}");
    }

    #[test]
    fn round_trips_every_variant() {
        // Widths 1..=6 cover every row-padding case for 24-bit pixels.
        for w in 1..=6 {
            let img = sample(w, 3);
            for (bits, top_down) in [(24, false), (24, true), (32, false), (32, true)] {
                let path = temp_path(&format!("rt-{w}-{bits}-{top_down}.bmp"));
                std::fs::write(&path, encode_bmp(&img, bits, top_down)).unwrap();
                let got = load_bmp(&path).unwrap();
                std::fs::remove_file(&path).ok();
                let want = if bits == 24 {
                    opaque(img.clone())
                } else {
                    img.clone()
                };
                assert_eq!(got, want, "width {w}, {bits}-bit, top-down {top_down}");
            }
        }
    }

    #[test]
    fn save_bmp_writes_24_bit_bottom_up() {
        let path = temp_path("saved.bmp");
        save_bmp(&path, &sample(7, 5)).unwrap();
        let info = read_info(&path).unwrap();
        let got = load_bmp(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert!(info.bottom_up);
        assert_eq!(info.bytes_per_pixel, 3);
        assert_eq!(got, opaque(sample(7, 5)));
    }

    #[test]
    fn bi_rgb_32_bit_ignores_the_fourth_byte() {
        // 2×1, BI_RGB, rows padded to 8 bytes already.
        let mut bytes = header(2, 1, 32, 0);
        bytes.extend([1, 2, 3, 7, 4, 5, 6, 9]);
        let path = temp_path("rgb32.bmp");
        std::fs::write(&path, bytes).unwrap();
        let got = load_bmp(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(got.get_pixel(0, 0), &Rgba([3, 2, 1, 255]));
        assert_eq!(got.get_pixel(1, 0), &Rgba([6, 5, 4, 255]));
    }

    #[test]
    fn read_rect_matches_a_crop() {
        let img = sample(9, 7);
        for (bits, top_down) in [(24, false), (32, true)] {
            let path = temp_path(&format!("rect-{bits}.bmp"));
            std::fs::write(&path, encode_bmp(&img, bits, top_down)).unwrap();
            let info = read_info(&path).unwrap();
            let full = load_bmp(&path).unwrap();
            let mut file = File::open(&path).unwrap();
            let got = read_rect(&mut file, &info, Rect::new(2, 1, 6, 5)).unwrap();
            assert!(read_rect(&mut file, &info, Rect::new(5, 5, 9, 6)).is_err());
            drop(file);
            std::fs::remove_file(&path).ok();
            let want = image::imageops::crop_imm(&full, 2, 1, 5, 5).to_image();
            assert_eq!(got, want, "{bits}-bit");
        }
    }

    #[test]
    fn write_rect_changes_only_that_rectangle() {
        let img = sample(9, 7);
        for (bits, top_down) in [(24, false), (32, true)] {
            let path = temp_path(&format!("write-{bits}.bmp"));
            let before = encode_bmp(&img, bits, top_down);
            std::fs::write(&path, &before).unwrap();
            let info = read_info(&path).unwrap();
            let patch = RgbaImage::from_pixel(3, 2, Rgba([10, 20, 30, 255]));
            let mut file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            write_rect(&mut file, &info, 4, 3, &patch).unwrap();
            drop(file);
            let after = std::fs::read(&path).unwrap();
            let got = load_bmp(&path).unwrap();
            std::fs::remove_file(&path).ok();
            assert_eq!(after.len(), before.len());
            let bpp = u64::from(info.bytes_per_pixel);
            for (i, (a, b)) in before.iter().zip(&after).enumerate() {
                if a != b {
                    let i = i as u64;
                    let inside = (3..5).any(|y| {
                        let start = info.row_offset(y) + 4 * bpp;
                        (start..start + 3 * bpp).contains(&i)
                    });
                    assert!(inside, "byte {i} changed outside the rectangle");
                }
            }
            for y in 0..7 {
                for x in 0..9 {
                    let want = if (4..7).contains(&x) && (3..5).contains(&y) {
                        Rgba([10, 20, 30, 255])
                    } else {
                        let p = *img.get_pixel(x, y);
                        if bits == 24 {
                            Rgba([p[0], p[1], p[2], 255])
                        } else {
                            p
                        }
                    };
                    assert_eq!(got.get_pixel(x, y), &want, "({x},{y}) {bits}-bit");
                }
            }
        }
    }
}
