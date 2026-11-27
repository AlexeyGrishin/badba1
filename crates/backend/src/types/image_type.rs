//! The `image` type: the default for fields without a recognized mark. Its
//! data area is a plain bitmap with no value; a write places a bitmap at the
//! data area's top-left.

use super::{AreaView, FieldType, Input, InputKind, Mark, Value};
use crate::detect::FRAME_COLOR;
use crate::draw::{Plan, WriteError};
use crate::layout::Field;

/// The `image` field type.
pub struct Image;

impl FieldType for Image {
    fn name(&self) -> &'static str {
        "image"
    }

    fn mark(&self) -> Option<Mark> {
        None
    }

    fn read(&self, area: &AreaView<'_>, _color: [u8; 4]) -> Value {
        Value::Bitmap {
            width: area.width(),
            height: area.height(),
        }
    }

    fn input_kind(&self) -> Option<InputKind> {
        Some(InputKind::Bitmap)
    }

    fn plan(&self, f: &Field, input: Input) -> Result<Plan, WriteError> {
        let Input::Bitmap(bitmap) = input else {
            unreachable!("checked against input_kind");
        };
        let (w, h) = (f.inner.width(), f.inner.height());
        let (bw, bh) = bitmap.dimensions();
        if bw > w || bh > h {
            return Err(WriteError::FieldOverflow {
                field: f.id,
                reason: format!("image is {bw}x{bh}, data area is {w}x{h}"),
            });
        }
        if let Some((x, y, _)) = bitmap
            .enumerate_pixels()
            .find(|(_, _, p)| **p == FRAME_COLOR)
        {
            return Err(WriteError::FrameColor { field: f.id, x, y });
        }
        Ok(Plan::Bitmap(bitmap))
    }

    fn cli_text(&self, _value: &Value) -> String {
        "image".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::tests::fill;
    use crate::layout::Rect;
    use crate::types::testing::*;
    use image::{Rgba, RgbaImage};

    fn bitmap(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| Rgba([(x * 7) as u8, (y * 11) as u8, 50, 255]))
    }

    #[test]
    fn value_is_the_data_area_size() {
        let img = scene(33, 20, None);
        assert_eq!(
            data_of(&img),
            (
                "image",
                Value::Bitmap {
                    width: 33,
                    height: 20
                }
            )
        );
        assert_eq!(Image.cli_text(&data_of(&img).1), "image");
    }

    #[test]
    fn smaller_image() {
        let mut img = scene(33, 20, None);
        fill(&mut img, Rect::new(30, 25, 40, 30), RED); // old content
        let bmp = bitmap(10, 10);
        assert_eq!(
            write(&mut img, Input::Bitmap(bmp.clone())).unwrap(),
            Value::Bitmap {
                width: 33,
                height: 20
            }
        );
        for y in 0..20 {
            for x in 0..33 {
                let p = *img.get_pixel(13 + x, 13 + y);
                let want = if x < 10 && y < 10 {
                    *bmp.get_pixel(x, y)
                } else {
                    WHITE
                };
                assert_eq!(p, want, "({x},{y})");
            }
        }
    }

    #[test]
    fn exact_size() {
        let mut img = scene(33, 20, None);
        let bmp = bitmap(33, 20);
        write(&mut img, Input::Bitmap(bmp.clone())).unwrap();
        for (x, y, p) in bmp.enumerate_pixels() {
            assert_eq!(img.get_pixel(13 + x, 13 + y), p);
        }
    }

    #[test]
    fn too_large() {
        let err = fails(scene(33, 20, None), Input::Bitmap(bitmap(34, 20)));
        assert!(matches!(err, WriteError::FieldOverflow { .. }));
    }

    #[test]
    fn frame_color_in_the_bitmap() {
        let mut bmp = bitmap(10, 10);
        bmp.put_pixel(4, 7, FRAME_COLOR);
        let err = fails(scene(33, 20, None), Input::Bitmap(bmp));
        assert!(matches!(err, WriteError::FrameColor { x: 4, y: 7, .. }));
    }
}
