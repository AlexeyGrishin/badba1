//! The bitmap font used to read `string` fields.
//!
//! A font is data: an image of 3×5 glyph cells (`font/font.png`) and the
//! characters of its first row (`font/chars.txt`). Cell column `c` starts at
//! x = 4·c and cell row `r` at y = 6·r. Row 0 holds each character's main form;
//! a non-empty cell lower in the same column is an alternate form of that
//! character. The top-left pixel's color is the background.

use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

/// Glyph cell width in pixels.
pub const GLYPH_WIDTH: u32 = 3;
/// Glyph cell height in pixels.
pub const GLYPH_HEIGHT: u32 = 5;

/// A glyph shape: bit `y * 3 + x` is set when pixel (x, y) of the 3×5 cell is
/// ink, rows top to bottom.
pub type GlyphMask = u16;

/// The bit for pixel (`x`, `y`) of a glyph cell.
pub const fn glyph_bit(x: u32, y: u32) -> GlyphMask {
    1 << (y * GLYPH_WIDTH + x)
}

/// A parsed font: every glyph form mapped to its character.
#[derive(Debug, Clone)]
pub struct Font {
    chars: Vec<char>,
    glyphs: HashMap<GlyphMask, char>,
    forms: Vec<(char, GlyphMask)>,
}

/// Why a font could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontError {
    /// The font image is not a valid PNG.
    Decode(String),
    /// `chars.txt` and the non-empty cells of row 0 differ in number.
    CountMismatch { chars: usize, cells: usize },
    /// A row-0 cell before the last character's column is empty.
    EmptyMainCell { column: usize },
    /// The same shape belongs to two different characters.
    SharedShape { first: char, second: char },
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontError::Decode(e) => write!(f, "font image is not a valid PNG: {e}"),
            FontError::CountMismatch { chars, cells } => write!(
                f,
                "font lists {chars} characters but its first row has {cells} glyphs"
            ),
            FontError::EmptyMainCell { column } => {
                write!(f, "font cell in column {column} of the first row is empty")
            }
            FontError::SharedShape { first, second } => write!(
                f,
                "font characters '{first}' and '{second}' share the same glyph shape"
            ),
        }
    }
}

impl std::error::Error for FontError {}

impl Font {
    /// Parses a font from its PNG image and its character list.
    pub fn parse(png: &[u8], chars: &str) -> Result<Font, FontError> {
        let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
            .map_err(|e| FontError::Decode(e.to_string()))?
            .to_rgba8();
        let chars: Vec<char> = chars.trim_end_matches(['\r', '\n']).chars().collect();
        let (w, h) = image.dimensions();
        let background = *image.get_pixel(0, 0);
        let columns = if w >= GLYPH_WIDTH {
            ((w - GLYPH_WIDTH) / 4 + 1) as usize
        } else {
            0
        };
        let rows = if h >= GLYPH_HEIGHT {
            ((h - GLYPH_HEIGHT) / 6 + 1) as usize
        } else {
            0
        };
        let cell = |c: usize, r: usize| -> GlyphMask {
            let (x0, y0) = (c as u32 * 4, r as u32 * 6);
            let mut mask = 0;
            for y in 0..GLYPH_HEIGHT {
                for x in 0..GLYPH_WIDTH {
                    if *image.get_pixel(x0 + x, y0 + y) != background {
                        mask |= glyph_bit(x, y);
                    }
                }
            }
            mask
        };

        let main_cells = (0..columns)
            .filter(|&c| rows > 0 && cell(c, 0) != 0)
            .count();
        if main_cells != chars.len() {
            return Err(FontError::CountMismatch {
                chars: chars.len(),
                cells: main_cells,
            });
        }
        if let Some(column) = (0..chars.len()).find(|&c| cell(c, 0) == 0) {
            return Err(FontError::EmptyMainCell { column });
        }

        let mut glyphs = HashMap::new();
        let mut forms = Vec::new();
        for (c, &ch) in chars.iter().enumerate() {
            for r in 0..rows {
                let mask = cell(c, r);
                if mask == 0 {
                    continue;
                }
                match glyphs.insert(mask, ch) {
                    Some(other) if other != ch => {
                        return Err(FontError::SharedShape {
                            first: other,
                            second: ch,
                        });
                    }
                    _ => forms.push((ch, mask)),
                }
            }
        }
        Ok(Font {
            chars,
            glyphs,
            forms,
        })
    }

    /// The character whose glyph form is exactly `mask`, if any.
    pub fn lookup(&self, mask: GlyphMask) -> Option<char> {
        self.glyphs.get(&mask).copied()
    }

    /// The font's characters, in the order of its first row.
    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    /// Every glyph form, main and alternate, with its character. A character's
    /// main form comes before its alternates.
    pub fn forms(&self) -> &[(char, GlyphMask)] {
        &self.forms
    }

    /// The main (first-row) glyph form of `ch`, if the font has it.
    pub fn main_form(&self, ch: char) -> Option<GlyphMask> {
        self.forms.iter().find(|(c, _)| *c == ch).map(|&(_, m)| m)
    }
}

/// The font built into the backend from `font/font.png` and `font/chars.txt`.
pub fn builtin() -> &'static Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(|| {
        Font::parse(
            include_bytes!("../font/font.png"),
            include_str!("../font/chars.txt"),
        )
        .expect("built-in font is valid")
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    /// Parses a 3×5 pattern given as five rows of `#`/`.` into a mask.
    pub fn mask(rows: [&str; 5]) -> GlyphMask {
        let mut m = 0;
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch == '#' {
                    m |= glyph_bit(x as u32, y as u32);
                }
            }
        }
        m
    }

    /// Encodes a font image from cells: `cells[r][c]` is the mask at (c, r).
    fn font_png(cells: &[Vec<GlyphMask>]) -> Vec<u8> {
        let cols = cells.iter().map(Vec::len).max().unwrap_or(0) as u32;
        let rows = cells.len() as u32;
        let mut img = RgbaImage::from_pixel(cols * 4 - 1, rows * 6 - 1, Rgba([255; 4]));
        for (r, row) in cells.iter().enumerate() {
            for (c, &m) in row.iter().enumerate() {
                for y in 0..GLYPH_HEIGHT {
                    for x in 0..GLYPH_WIDTH {
                        if m & glyph_bit(x, y) != 0 {
                            let p = (c as u32 * 4 + x, r as u32 * 6 + y);
                            img.put_pixel(p.0, p.1, Rgba([0, 0, 0, 255]));
                        }
                    }
                }
            }
        }
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn builtin_font_is_valid() {
        let font = builtin();
        assert_eq!(
            font.chars().iter().collect::<String>(),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890_+-*"
        );
        assert_eq!(font.forms().len(), 56);
    }

    #[test]
    fn both_e_forms_are_e() {
        let font = builtin();
        assert_eq!(
            font.lookup(mask(["###", "#..", "##.", "#..", "###"])),
            Some('E')
        );
        assert_eq!(
            font.lookup(mask(["###", "#..", "###", "#..", "###"])),
            Some('E')
        );
        assert_eq!(
            font.lookup(mask(["###", "#..", "##.", "#..", "#.."])),
            Some('F')
        );
    }

    #[test]
    fn cell_positions_follow_the_grid() {
        // Column 2 of row 1 covers x 8..=10, y 6..=10: an alternate for 'C'.
        let a = mask([".#.", "#.#", "###", "#.#", "#.#"]);
        let b = mask(["##.", "#.#", "##.", "#.#", "##."]);
        let c = mask(["###", "#..", "#..", "#..", "###"]);
        let c2 = mask([".##", "#..", "#..", "#..", ".##"]);
        let png = font_png(&[vec![a, b, c], vec![0, 0, c2]]);
        let font = Font::parse(&png, "ABC\n").unwrap();
        assert_eq!(font.lookup(c2), Some('C'));
        assert_eq!(font.forms().len(), 4);
    }

    #[test]
    fn shared_shape_is_rejected() {
        let a = mask([".#.", "#.#", "###", "#.#", "#.#"]);
        let b = mask(["##.", "#.#", "##.", "#.#", "##."]);
        let png = font_png(&[vec![a, b], vec![0, a]]);
        let err = Font::parse(&png, "AB").unwrap_err();
        assert_eq!(
            err,
            FontError::SharedShape {
                first: 'A',
                second: 'B'
            }
        );
        let text = err.to_string();
        assert!(text.contains('A') && text.contains('B'), "{text}");
    }

    #[test]
    fn count_mismatch_is_rejected() {
        let a = mask([".#.", "#.#", "###", "#.#", "#.#"]);
        let b = mask(["##.", "#.#", "##.", "#.#", "##."]);
        let png = font_png(&[vec![a, b]]);
        let err = Font::parse(&png, "A").unwrap_err();
        assert_eq!(err, FontError::CountMismatch { chars: 1, cells: 2 });
        let text = err.to_string();
        assert!(text.contains('1') && text.contains('2'), "{text}");
    }
}
