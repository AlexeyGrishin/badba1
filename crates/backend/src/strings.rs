//! Reading the text of `string` fields.
//!
//! Ink is every data-area pixel whose color differs from the image's top-left
//! pixel. Letters are 3×5 cells matched exactly against the glyph [`Font`].
//! A line's top row comes from letters whose ink is exactly 5 rows tall;
//! shorter marks join the line whose rows contain them.

use std::collections::{BTreeMap, BTreeSet};

use image::RgbaImage;

use crate::font::{Font, GLYPH_HEIGHT, GLYPH_WIDTH, glyph_bit};
use crate::layout::Rect;

/// The character for a letter cell that matches no glyph in the font.
pub const UNKNOWN_GLYPH: char = '\u{FFFD}';

/// Reads the text written in `area` of `image`, using `font`. Lines are
/// ordered top to bottom (then left to right) and joined with `\n`.
pub fn read_text(image: &RgbaImage, area: Rect, font: &Font) -> String {
    let background = *image.get_pixel(0, 0);
    let (w, h) = (area.width() as usize, area.height() as usize);
    let ink: Vec<bool> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| *image.get_pixel(area.left + x as u32, area.top + y as u32) != background)
        .collect();

    // 8-connected ink components, with bounding rows.
    let mut label = vec![usize::MAX; w * h];
    let mut comps: Vec<(usize, usize)> = Vec::new(); // (top, bottom) rows
    for start in 0..w * h {
        if !ink[start] || label[start] != usize::MAX {
            continue;
        }
        let id = comps.len();
        let (mut top, mut bottom) = (start / w, start / w);
        label[start] = id;
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            top = top.min(y);
            bottom = bottom.max(y);
            for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let j = ny * w + nx;
                    if ink[j] && label[j] == usize::MAX {
                        label[j] = id;
                        stack.push(j);
                    }
                }
            }
        }
        comps.push((top, bottom));
    }

    // Line tops from letters exactly 5 rows tall; every component joins the
    // closest line above it whose 5 rows contain it, or starts its own line.
    let anchors: BTreeSet<usize> = comps
        .iter()
        .filter(|&&(t, b)| b - t + 1 == GLYPH_HEIGHT as usize)
        .map(|&(t, _)| t)
        .collect();
    let line_of: Vec<usize> = comps
        .iter()
        .map(|&(t, b)| {
            anchors
                .range(..=t)
                .next_back()
                .copied()
                .filter(|&top| b < top + GLYPH_HEIGHT as usize)
                .unwrap_or(t)
        })
        .collect();
    let mut lines: BTreeMap<usize, Vec<bool>> = BTreeMap::new(); // top -> members
    for (id, &top) in line_of.iter().enumerate() {
        lines.entry(top).or_insert_with(|| vec![false; comps.len()])[id] = true;
    }

    let mut texts: Vec<(usize, usize, String)> = Vec::new(); // (top, left, text)
    for (&top, members) in &lines {
        let in_line = |x: usize, y: usize| {
            y < h && x < w && label[y * w + x] != usize::MAX && members[label[y * w + x]]
        };
        let rows = top..(top + GLYPH_HEIGHT as usize).min(h);
        let occupied = |x: usize| rows.clone().any(|y| in_line(x, y));
        let mut text = String::new();
        let mut left = None;
        let mut prev_end: Option<usize> = None;
        let mut x = 0;
        while x < w {
            if !occupied(x) {
                x += 1;
                continue;
            }
            if let Some(end) = prev_end {
                let gap = x - end;
                text.extend(std::iter::repeat_n(' ', gap / GLYPH_WIDTH as usize));
            }
            left.get_or_insert(x);
            let mut mask = 0;
            for cy in 0..GLYPH_HEIGHT {
                for cx in 0..GLYPH_WIDTH {
                    if in_line(x + cx as usize, top + cy as usize) {
                        mask |= glyph_bit(cx, cy);
                    }
                }
            }
            text.push(font.lookup(mask).unwrap_or(UNKNOWN_GLYPH));
            prev_end = Some(x + GLYPH_WIDTH as usize);
            x += GLYPH_WIDTH as usize;
        }
        if let Some(left) = left {
            texts.push((top, left, text));
        }
    }
    texts.sort_by_key(|&(top, left, _)| (top, left));
    texts
        .into_iter()
        .map(|(_, _, t)| t)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::builtin;
    use crate::font::tests::mask;
    use image::Rgba;

    const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
    const BLUE: Rgba<u8> = Rgba([0, 162, 232, 255]);
    const PINK: Rgba<u8> = Rgba([255, 174, 201, 255]);
    const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);

    /// The data area used by the tests: the whole 100×40 image but its border.
    const AREA: Rect = Rect::new(1, 1, 98, 38);

    fn canvas() -> RgbaImage {
        RgbaImage::from_pixel(100, 40, WHITE)
    }

    /// The `k`-th form (0 = main) of `ch` in the built-in font.
    fn form(ch: char, k: usize) -> u16 {
        builtin()
            .forms()
            .iter()
            .filter(|(c, _)| *c == ch)
            .nth(k)
            .unwrap_or_else(|| panic!("no form {k} of {ch:?}"))
            .1
    }

    fn stamp(img: &mut RgbaImage, x: u32, y: u32, m: u16, color: Rgba<u8>) {
        for cy in 0..GLYPH_HEIGHT {
            for cx in 0..GLYPH_WIDTH {
                if m & glyph_bit(cx, cy) != 0 {
                    img.put_pixel(x + cx, y + cy, color);
                }
            }
        }
    }

    /// Writes `text` (main forms) starting at (x, y), with `gap` empty columns
    /// between letters.
    fn write(img: &mut RgbaImage, x: u32, y: u32, text: &str, gap: u32, color: Rgba<u8>) {
        for (i, ch) in text.chars().enumerate() {
            stamp(
                img,
                x + i as u32 * (GLYPH_WIDTH + gap),
                y,
                form(ch, 0),
                color,
            );
        }
    }

    fn read(img: &RgbaImage) -> String {
        read_text(img, AREA, builtin())
    }

    #[test]
    fn letters_in_several_colors() {
        let mut img = canvas();
        write(&mut img, 2, 2, "OPEN", 1, BLUE);
        write(&mut img, 18, 2, "S", 1, PINK);
        write(&mut img, 22, 2, "PEC", 1, BLUE);
        assert_eq!(read(&img), "OPENSPEC");
    }

    #[test]
    fn non_white_background() {
        let mut img = RgbaImage::from_pixel(100, 40, Rgba([200, 200, 200, 255]));
        write(&mut img, 2, 2, "AB", 1, BLACK);
        assert_eq!(read(&img), "AB");
    }

    #[test]
    fn one_column_gaps() {
        let mut img = canvas();
        write(&mut img, 2, 2, "ABC", 1, BLACK);
        assert_eq!(read(&img), "ABC");
    }

    #[test]
    fn touching_letters() {
        let mut img = canvas();
        write(&mut img, 2, 2, "HI", 0, BLACK);
        assert_eq!(read(&img), "HI");
    }

    #[test]
    fn two_lines() {
        let mut img = canvas();
        write(&mut img, 2, 10, "AB", 1, BLACK);
        write(&mut img, 2, 17, "CD", 1, BLACK);
        assert_eq!(read(&img), "AB\nCD");
    }

    #[test]
    fn short_glyph_in_a_line() {
        let mut img = canvas();
        write(&mut img, 2, 2, "A-B", 1, BLACK);
        assert_eq!(read(&img), "A-B");
    }

    #[test]
    fn multi_line_text() {
        let mut img = canvas();
        write(&mut img, 2, 2, "TOP", 1, BLACK);
        write(&mut img, 2, 12, "BOTTOM", 1, BLACK);
        assert_eq!(read(&img), "TOP\nBOTTOM");
    }

    #[test]
    fn lines_beside_each_other() {
        let mut img = canvas();
        write(&mut img, 2, 5, "LEFT", 1, BLACK);
        write(&mut img, 60, 12, "RIGHT", 1, BLACK);
        assert_eq!(read(&img), "LEFT\nRIGHT");
    }

    #[test]
    fn empty_field() {
        assert_eq!(read(&canvas()), "");
    }

    #[test]
    fn both_forms_of_e() {
        let mut img = canvas();
        stamp(&mut img, 2, 2, form('E', 0), BLACK);
        stamp(&mut img, 6, 2, form('E', 1), BLACK);
        assert_ne!(form('E', 0), form('E', 1));
        assert_eq!(read(&img), "EE");
    }

    #[test]
    fn unknown_glyph() {
        let block = mask(["###", "###", "###", "###", "###"]);
        assert_eq!(builtin().lookup(block), None);
        let mut img = canvas();
        write(&mut img, 2, 2, "A", 1, BLACK);
        stamp(&mut img, 6, 2, block, BLACK);
        assert_eq!(read(&img), "A\u{FFFD}");
    }

    #[test]
    fn one_pixel_off() {
        // The main A with one extra pixel in its top-left corner.
        assert_eq!(form('A', 0) & glyph_bit(0, 0), 0);
        let one_off = form('A', 0) | glyph_bit(0, 0);
        assert_eq!(builtin().lookup(one_off), None);
        let mut img = canvas();
        stamp(&mut img, 2, 2, one_off, BLACK);
        assert_eq!(read(&img), "\u{FFFD}");
    }

    fn pair_with_gap(gap: u32) -> String {
        let mut img = canvas();
        write(&mut img, 2, 2, "AB", gap, BLACK);
        read(&img)
    }

    #[test]
    fn small_gaps_are_not_spaces() {
        assert_eq!(pair_with_gap(1), "AB");
        assert_eq!(pair_with_gap(2), "AB");
    }

    #[test]
    fn one_space() {
        for gap in [3, 4, 5] {
            assert_eq!(pair_with_gap(gap), "A B", "gap {gap}");
        }
    }

    #[test]
    fn two_spaces() {
        assert_eq!(pair_with_gap(6), "A  B");
    }

    #[test]
    fn leading_and_trailing_gaps() {
        let mut img = canvas();
        write(&mut img, 11, 2, "AB", 1, BLACK);
        assert_eq!(read(&img), "AB");
    }
}
