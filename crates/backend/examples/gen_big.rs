//! Generates a large synthetic badba1 database image.
//!
//! Usage: `cargo run --release -p backend --example gen_big -- <out.png> [photo.png|- [width height records]]`
//! (defaults: no photo, 10000 40000 9000). With a photo, each `image` field
//! holds a random crop of it at a random scale.
//!
//! Records sit on a grid of 1000×44 cells, one record per cell, in reading
//! order. Each record has 8 fields chained by frame-colored links:
//! string, counter, float, bool, ref, string, image, counter. A `ref` field
//! points to the record directly above it (some are left null).

use std::path::Path;

use image::{Rgb, RgbImage};

const CELL_W: u32 = 1000;
const CELL_H: u32 = 44;
const FIELD_TOP: u32 = 7; // outer top within the cell (mark rows are 4..=6)
const FIELD_H: u32 = 31;
const BORDER: u32 = 2;
const GAP: u32 = 10;
const LEFT: u32 = 5;

const WHITE: Rgb<u8> = Rgb([255, 255, 255]);
const FRAME: Rgb<u8> = Rgb([186, 219, 161]);
const RED: Rgb<u8> = Rgb([237, 28, 36]);
const GREEN: Rgb<u8> = Rgb([34, 177, 76]);
const PINK: Rgb<u8> = Rgb([255, 174, 201]);
const YELLOW: Rgb<u8> = Rgb([255, 201, 14]);
const BLACK: Rgb<u8> = Rgb([0, 0, 0]);
const BLUE: Rgb<u8> = Rgb([63, 72, 204]);

#[derive(Clone, Copy)]
enum Kind {
    Str(Rgb<u8>),
    Counter,
    Float,
    Bool,
    Ref,
    Image,
}

const FIELDS: [(Kind, u32); 8] = [
    (Kind::Str(BLACK), 200),
    (Kind::Counter, 100),
    (Kind::Float, 100),
    (Kind::Bool, 40),
    (Kind::Ref, 40),
    (Kind::Str(BLUE), 200),
    (Kind::Image, 120),
    (Kind::Counter, 100),
];

const STRING_MARK: [&str; 3] = [".#.", "#.#", "#.#"];
const COUNTER_MARK: [&str; 3] = ["###", "#..", "###"];
const FLOAT_MARK: [&str; 3] = ["###", "##.", "#.."];
const BOOL_MARK: [&str; 3] = [".#.", ".##", ".##"];
const REF_MARK: [&str; 3] = [".#.", "###", ".#."];

const WORDS: [&str; 16] = [
    "ALPHA", "BRAVO", "CHARLIE", "DELTA", "ECHO", "FOXTROT", "GOLF", "HOTEL", "INDIA", "JULIET",
    "KILO", "LIMA", "MIKE", "NOVEMBER", "OSCAR", "PAPA",
];
const CODE_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890_+-*";

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn fill(img: &mut RgbImage, x: u32, y: u32, w: u32, h: u32, c: Rgb<u8>) {
    for yy in y..y + h {
        for xx in x..x + w {
            img.put_pixel(xx, yy, c);
        }
    }
}

fn draw_mark(img: &mut RgbImage, right: u32, top: u32, shape: [&str; 3], c: Rgb<u8>) {
    for (dy, row) in shape.iter().enumerate() {
        for (dx, ch) in row.bytes().enumerate() {
            if ch == b'#' {
                img.put_pixel(right - 2 + dx as u32, top - 3 + dy as u32, c);
            }
        }
    }
}

/// Draws text like a `string` write: main glyph forms, 1 empty column between
/// letters, a space adds 3 columns, `\n` moves 6 rows down.
fn draw_text(img: &mut RgbImage, x0: u32, y0: u32, text: &str, c: Rgb<u8>) {
    let font = backend::builtin_font();
    let (mut x, mut y) = (x0, y0);
    for ch in text.chars() {
        match ch {
            '\n' => {
                x = x0;
                y += 6;
            }
            ' ' => x += 3,
            _ => {
                let mask = font.main_form(ch).expect("char in font");
                for gy in 0..5 {
                    for gx in 0..3 {
                        if mask & backend::GlyphMask::from(1u16 << (gy * 3 + gx)) != 0 {
                            img.put_pixel(x + gx, y + gy, c);
                        }
                    }
                }
                x += 4;
            }
        }
    }
}

/// Draws N squares like a `counter` write: largest square size whose grid
/// (1-pixel gaps) holds N, filled in reading order.
fn draw_counter(img: &mut RgbImage, x0: u32, y0: u32, w: u32, h: u32, n: u32, c: Rgb<u8>) {
    if n == 0 {
        return;
    }
    let mut s = w.min(h);
    while s > 1 && ((w + 1) / (s + 1)) * ((h + 1) / (s + 1)) < n {
        s -= 1;
    }
    let cols = (w + 1) / (s + 1);
    assert!(cols * ((h + 1) / (s + 1)) >= n, "counter overflow");
    for i in 0..n {
        let (gx, gy) = (i % cols, i / cols);
        fill(img, x0 + gx * (s + 1), y0 + gy * (s + 1), s, s, c);
    }
}

fn draw_record(
    img: &mut RgbImage,
    rng: &mut Rng,
    photos: &[RgbImage],
    index: u32,
    cx: u32,
    cy: u32,
    has_above: bool,
) {
    let top = cy + FIELD_TOP;
    let mut x = cx + LEFT;
    let mut prev_right: Option<u32> = None;
    for (fi, &(kind, w)) in FIELDS.iter().enumerate() {
        let right = x + w - 1;
        // Frame and data area.
        fill(img, x, top, w, FIELD_H, FRAME);
        let (dx, dy, dw, dh) = (
            x + BORDER,
            top + BORDER,
            w - 2 * BORDER,
            FIELD_H - 2 * BORDER,
        );
        fill(img, dx, dy, dw, dh, WHITE);
        // Link to the previous field.
        if let Some(pr) = prev_right {
            fill(img, pr + 1, top + FIELD_H / 2 - 1, x - pr - 1, 2, FRAME);
        }
        match kind {
            Kind::Str(c) => {
                draw_mark(img, right, top, STRING_MARK, c);
                let text = if fi == 0 {
                    format!("REC {:04}\n{}", index + 1, WORDS[rng.below(16) as usize])
                } else {
                    let len = 4 + rng.below(20) as usize;
                    let mut s: String = (0..len)
                        .map(|_| CODE_CHARS[rng.below(CODE_CHARS.len() as u64) as usize] as char)
                        .collect();
                    if rng.below(2) == 0 {
                        s.push('\n');
                        s.push_str(WORDS[rng.below(16) as usize]);
                        s.push(' ');
                        s.push_str(WORDS[rng.below(16) as usize]);
                    }
                    s
                };
                draw_text(img, dx, dy, &text, c);
            }
            Kind::Counter => {
                draw_mark(img, right, top, COUNTER_MARK, RED);
                let n = rng.below(60) as u32;
                draw_counter(img, dx, dy, dw, dh, n, RED);
            }
            Kind::Float => {
                draw_mark(img, right, top, FLOAT_MARK, GREEN);
                let pct = rng.below(10001) as f64 / 100.0;
                let area = dw * dh;
                let n = (pct * area as f64 / 100.0).round() as u32;
                for i in 0..n {
                    img.put_pixel(dx + i / dh, dy + i % dh, GREEN);
                }
            }
            Kind::Bool => {
                draw_mark(img, right, top, BOOL_MARK, PINK);
                if rng.below(2) == 0 {
                    fill(img, dx, dy, dw, dh, PINK);
                }
            }
            Kind::Ref => {
                draw_mark(img, right, top, REF_MARK, YELLOW);
                // Line up from the mark's top pixel to the bottom frame of the
                // same field in the record above; ~1 in 8 stays null.
                if has_above && rng.below(8) != 0 {
                    let col = right - 1;
                    let above_bottom = top - CELL_H + FIELD_H - 1;
                    for yy in above_bottom + 1..top - 3 {
                        img.put_pixel(col, yy, YELLOW);
                    }
                }
            }
            Kind::Image if !photos.is_empty() => {
                // No mark: a random crop of the photo at a random scale.
                let p = &photos[rng.below(photos.len() as u64) as usize];
                let ox = rng.below((p.width() - dw + 1) as u64) as u32;
                let oy = rng.below((p.height() - dh + 1) as u64) as u32;
                for yy in 0..dh {
                    for xx in 0..dw {
                        img.put_pixel(dx + xx, dy + yy, *p.get_pixel(ox + xx, oy + yy));
                    }
                }
            }
            Kind::Image => {
                // No mark: a few colored rectangles and a diagonal.
                for _ in 0..1 + rng.below(5) {
                    let c = Rgb([rng.below(256) as u8, rng.below(256) as u8, 0]);
                    let rw = 2 + rng.below(dw as u64 / 2) as u32;
                    let rh = 2 + rng.below(dh as u64 / 2) as u32;
                    let rx = rng.below((dw - rw) as u64) as u32;
                    let ry = rng.below((dh - rh) as u64) as u32;
                    fill(img, dx + rx, dy + ry, rw, rh, c);
                }
                for i in 0..dh {
                    img.put_pixel(dx + i * 4, dy + i, BLACK);
                }
            }
        }
        prev_right = Some(right);
        x = right + 1 + GAP;
    }
}

/// Loads the photo for `image` fields at several scales (widths from the
/// image field's data width up to the original), or none for `-`. Pixels in
/// the frame color are nudged so the photo never forms frames or links.
fn load_photos(path: &str) -> Vec<RgbImage> {
    if path == "-" {
        return Vec::new();
    }
    let src = image::open(path).unwrap().to_rgb8();
    let min_w = FIELDS
        .iter()
        .find(|f| matches!(f.0, Kind::Image))
        .unwrap()
        .1
        - 2 * BORDER;
    let mut out = Vec::new();
    for w in [min_w, min_w * 3 / 2, min_w * 2, min_w * 3, src.width()] {
        let w = w.min(src.width());
        let h = (src.height() as u64 * w as u64 / src.width() as u64) as u32;
        let mut p = image::imageops::resize(&src, w, h, image::imageops::FilterType::Triangle);
        for px in p.pixels_mut() {
            if *px == FRAME {
                px.0[1] -= 1;
            }
        }
        out.push(p);
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = args.first().map(String::as_str).unwrap_or("db/big.png");
    let photo = args.get(1).map(String::as_str).unwrap_or("-");
    let num = |i: usize, d: u32| args.get(i).map(|s| s.parse().unwrap()).unwrap_or(d);
    let (width, height, records) = (num(2, 10000), num(3, 40000), num(4, 9000));
    let photos = load_photos(photo);
    let cols = width / CELL_W;
    let rows = records.div_ceil(cols);
    assert!(
        rows * CELL_H <= height,
        "{records} records need {} rows",
        rows * CELL_H
    );

    let mut img = RgbImage::from_pixel(width, height, WHITE);
    let mut rng = Rng(0x00BA_DBA1_5EED);
    for i in 0..records {
        let (c, r) = (i % cols, i / cols);
        draw_record(
            &mut img,
            &mut rng,
            &photos,
            i,
            c * CELL_W,
            r * CELL_H,
            r > 0,
        );
    }
    if let Some(dir) = Path::new(out).parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    img.save(out).unwrap();
    println!("wrote {out}: {width}x{height}, {records} records");
}
