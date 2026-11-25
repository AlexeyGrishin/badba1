# field-writes Specification

## Purpose

Defines how a value is written into a field of a badba1 image: which values each type accepts, how they are drawn in the data area, what the write returns, and when it fails. Field geometry is never changed.

## Requirements

### Requirement: Write a field's value
The backend SHALL write a value into one field, identified by its number in the image's detected layout. It changes pixels only inside that field's data area, and returns the field's value as read back from the modified image.

#### Scenario: Write and read back
- **WHEN** the value 3 is written to a `counter` field
- **THEN** the write returns `counter 3`, and analyzing the modified image gives the same fields, links and records as before, with that field's value now 3

#### Scenario: Pixels outside the data area are untouched
- **WHEN** any write succeeds
- **THEN** every pixel outside the written field's data area is unchanged

### Requirement: Clear and draw colors
A write SHALL first set every data-area pixel to the background color, which is the color of the image's top-left pixel. It then draws the new value in the field's mark color.

#### Scenario: Old content is removed
- **WHEN** a `counter` field holding 5 dots is written with 2
- **THEN** the data area holds only the 2 new squares, in the mark color, on the background color

### Requirement: Failed writes change nothing
A write that fails SHALL return an error and leave the image unchanged.

#### Scenario: Error leaves image as it was
- **WHEN** a write fails for any reason, such as an overflow, an unknown character or a wrong type
- **THEN** the image's pixels are identical to before the write

### Requirement: Wrong field or type
A write SHALL fail when no field has the given number, when the field is a `ref` (references are not writable), or when the value's type differs from the field's type.

#### Scenario: Unknown field
- **WHEN** a value is written to field number 99 in an image with 11 fields
- **THEN** the write fails with an error naming field 99

#### Scenario: Reference field
- **WHEN** any value is written to a `ref` field
- **THEN** the write fails with an error saying the field is not writable

#### Scenario: Type mismatch
- **WHEN** a text value is written to a `counter` field
- **THEN** the write fails with an error naming both types

### Requirement: Counter writes
Writing N to a `counter` field SHALL draw N separate squares of one size on a grid, with 1-pixel gaps, filled in reading order from the data area's top-left. It uses the largest square size whose grid holds at least N squares. Writing 0 clears the area. When even 1×1 squares can't hold N, the write fails with **FieldOverflow**.

#### Scenario: Squares in reading order
- **WHEN** 3 is written to a `counter` field with a 27×16 data area
- **THEN** 3 equal squares are drawn left to right on the grid's first row, and the field reads as `counter 3`

#### Scenario: Zero
- **WHEN** 0 is written to a `counter` field
- **THEN** the data area is cleared, and the field reads as `counter 0`

#### Scenario: Largest that fits
- **WHEN** 112 is written to a `counter` field with a 27×16 data area
- **THEN** 112 squares of 1×1 pixel are drawn, and the field reads as `counter 112`

#### Scenario: Overflow
- **WHEN** 113 is written to a `counter` field with a 27×16 data area
- **THEN** the write fails with FieldOverflow and the image is unchanged

### Requirement: Float writes
Writing N to a `float` field SHALL require 0 ≤ N ≤ 100. It fills round(N × area / 100) data-area pixels with the mark color, column by column from the left and top to bottom within a column, and returns the value actually stored.

#### Scenario: Stored value differs from requested
- **WHEN** 33.3 is written to a `float` field with a 28×19 data area (532 pixels)
- **THEN** 177 pixels are filled, and the write returns `float` 177/532, about 33.27%

#### Scenario: Exact value
- **WHEN** 50 is written to that field
- **THEN** 266 pixels are filled, and the write returns 50.00%

#### Scenario: Out of range
- **WHEN** −1 or 100.5 is written to a `float` field
- **THEN** the write fails with an error naming the value, and the image is unchanged

### Requirement: Bool writes
Writing true to a `bool` field SHALL fill its whole data area with the mark color. Writing false clears it.

#### Scenario: True and false
- **WHEN** false is written to a `bool` field that is true, and then true is written to it
- **THEN** it reads as `bool false` after the first write and `bool true` after the second

### Requirement: String writes
Writing text to a `string` field SHALL draw each character's main glyph form in the mark color. It starts at the data area's top-left, leaving one empty column between letters. A space adds 3 empty columns, and a line break moves to the left edge, 6 rows lower. Text never wraps.

#### Scenario: One line
- **WHEN** `HELLO` is written to a `string` field with a 40×5 data area
- **THEN** the letters start at columns 0, 4, 8, 12 and 16 of the data area, and the field reads as `HELLO`

#### Scenario: Space
- **WHEN** `A B` is written
- **THEN** there are 4 empty columns between `A` and `B`, and the field reads as `A B`

#### Scenario: Two lines
- **WHEN** `AB\nCD` is written to a `string` field whose data area is at least 11 rows tall
- **THEN** `CD` starts at the data area's left edge, 6 rows below `AB`, and the field reads as `AB\nCD`

### Requirement: String write errors
A string write SHALL fail with **UnknownCharacter** when the text holds a character that is neither in the glyph font nor a space or line break. It fails with **FieldOverflow** when a letter would extend past the data area's right or bottom edge.

#### Scenario: Unknown character
- **WHEN** `Hi` is written (lowercase `i` is not in the font)
- **THEN** the write fails with UnknownCharacter naming `i`, and the image is unchanged

#### Scenario: Too wide
- **WHEN** `HELLO WORLD` is written to a `string` field with a 40×5 data area
- **THEN** the write fails with FieldOverflow, because the last letter would cover columns 39–41 of a data area whose columns are 0–39

#### Scenario: Too tall
- **WHEN** `AB\nCD` is written to a `string` field with a 40×5 data area
- **THEN** the write fails with FieldOverflow

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
