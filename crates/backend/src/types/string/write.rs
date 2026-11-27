//! Writing text into `string` fields.

use crate::draw::WriteError;
use crate::font::{GLYPH_HEIGHT, GLYPH_WIDTH, builtin, glyph_bit};

/// Glyph pixels for `text`: 1-column letter gap, 3-column space, `\n` moves
/// down 6 rows to the left edge, no wrapping.
pub(super) fn string_pixels(
    field: usize,
    w: u32,
    h: u32,
    text: &str,
) -> Result<Vec<(u32, u32)>, WriteError> {
    let font = builtin();
    let (mut x, mut y) = (0u32, 0u32);
    let mut pixels = Vec::new();
    for ch in text.chars() {
        match ch {
            '\n' => {
                x = 0;
                y += GLYPH_HEIGHT + 1;
            }
            ' ' => x += GLYPH_WIDTH,
            _ => {
                let mask = font.main_form(ch).ok_or(WriteError::UnknownCharacter {
                    field,
                    character: ch,
                })?;
                if x + GLYPH_WIDTH > w || y + GLYPH_HEIGHT > h {
                    return Err(WriteError::FieldOverflow {
                        field,
                        reason: format!(
                            "letter {ch:?} at ({x},{y}) does not fit in the {w}x{h} data area"
                        ),
                    });
                }
                for cy in 0..GLYPH_HEIGHT {
                    for cx in 0..GLYPH_WIDTH {
                        if mask & glyph_bit(cx, cy) != 0 {
                            pixels.push((x + cx, y + cy));
                        }
                    }
                }
                x += GLYPH_WIDTH + 1;
            }
        }
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::super::STRING_MARK;
    use crate::draw::WriteError;
    use crate::types::testing::*;
    use crate::types::{Input, Value};

    fn text_of(v: Value) -> String {
        match v {
            Value::Text(text) => text,
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn one_line() {
        let mut img = scene(40, 5, Some(STRING_MARK));
        let got = write(&mut img, Input::Text("HELLO".into())).unwrap();
        assert_eq!(text_of(got), "HELLO");
        let px = data_pixels(&img, 40, 5);
        let starts: std::collections::BTreeSet<u32> = px.iter().map(|&(x, _)| x - x % 4).collect();
        assert_eq!(
            starts.into_iter().collect::<Vec<_>>(),
            vec![0, 4, 8, 12, 16]
        );
    }

    #[test]
    fn space() {
        let mut img = scene(40, 5, Some(STRING_MARK));
        let got = write(&mut img, Input::Text("A B".into())).unwrap();
        assert_eq!(text_of(got), "A B");
        let cols: std::collections::BTreeSet<u32> =
            data_pixels(&img, 40, 5).iter().map(|&(x, _)| x).collect();
        assert_eq!(cols.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 7, 8, 9]);
    }

    #[test]
    fn two_lines() {
        let mut img = scene(40, 11, Some(STRING_MARK));
        let got = write(&mut img, Input::Text("AB\nCD".into())).unwrap();
        assert_eq!(text_of(got), "AB\nCD");
        assert!(
            data_pixels(&img, 40, 11)
                .iter()
                .any(|&(x, y)| x == 0 && y >= 6)
        );
    }

    #[test]
    fn unknown_character() {
        let err = fails(scene(40, 5, Some(STRING_MARK)), Input::Text("Hi".into()));
        assert!(matches!(
            err,
            WriteError::UnknownCharacter { character: 'i', .. }
        ));
    }

    #[test]
    fn too_wide() {
        let err = fails(
            scene(40, 5, Some(STRING_MARK)),
            Input::Text("HELLO WORLD".into()),
        );
        assert!(matches!(err, WriteError::FieldOverflow { .. }));
    }

    #[test]
    fn too_tall() {
        let err = fails(scene(40, 5, Some(STRING_MARK)), Input::Text("AB\nCD".into()));
        assert!(matches!(err, WriteError::FieldOverflow { .. }));
    }
}
