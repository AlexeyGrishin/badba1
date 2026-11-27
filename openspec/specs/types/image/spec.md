# types/image Specification

## Purpose

Defines the `image` data type, the type of every field without a recognized type mark: a plain bitmap with no value, which is written by placing a bitmap in its data area.

## Requirements

### Requirement: Image value
An `image` field SHALL have no value. Its value kind SHALL be the size of its data area (width and height), and its CLI text SHALL be `image`.

#### Scenario: Image field of the reference image
- **WHEN** the field at (35,23)–(77,52) of `testdata/example1.png` is read
- **THEN** its type is `image`, it has no value, its size is its data area's width and height, and its CLI text is `image`

### Requirement: Image writes
Writing a bitmap to an `image` field SHALL place it at the data area's top-left after clearing the area, replacing the previous content. The bitmap must be no wider and no taller than the data area (otherwise **FieldOverflow**), and must not contain the frame color `#BADBA1`.

#### Scenario: Smaller image
- **WHEN** a 10×10 bitmap is written to an `image` field with a 33×20 data area
- **THEN** the data area's top-left 10×10 pixels equal the bitmap, and the rest is the background color

#### Scenario: Exact size
- **WHEN** a 33×20 bitmap is written to that field
- **THEN** the data area equals the bitmap

#### Scenario: Too large
- **WHEN** a 34×20 bitmap is written to an `image` field with a 33×20 data area
- **THEN** the write fails with FieldOverflow and the image is unchanged

#### Scenario: Frame color in the bitmap
- **WHEN** a bitmap containing a pixel of `#BADBA1` is written
- **THEN** the write fails with an error giving that pixel's position, and the image is unchanged
