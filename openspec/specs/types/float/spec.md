# types/float Specification

## Purpose

Defines the `float` data type: its type mark, how its value is read as the filled share of the data area, how a value is written, and how the value is shown.

## Requirements

### Requirement: Float mark
A field's type SHALL be `float` when its mark shape is `###` / `##.` / `#..` (rows top to bottom, `#` = mark color).

#### Scenario: Float mark
- **WHEN** the mark area holds green (34,177,76) pixels in the shape `###` / `##.` / `#..`, and white pixels elsewhere
- **THEN** the field's type is `float` and its mark color is (34,177,76)

#### Scenario: Other pixels are not the mark color
- **WHEN** the mark area holds a green `float` shape and the remaining pixels are a mix of white and black
- **THEN** the field's type is `float`

### Requirement: Float value
The value of a `float` field SHALL be the percentage of the data area's pixels that are exactly the mark color, from 0 to 100. Pixels of every other color count as empty.

#### Scenario: Partly filled area
- **WHEN** a `float` field with a green mark has a 10×10 data area in which 25 pixels are green, 10 are black and the rest are white
- **THEN** its value is 25%

#### Scenario: Empty and full areas
- **WHEN** a `float` field's data area has no pixels of the mark color, or only pixels of the mark color
- **THEN** its value is 0% or 100%, respectively

### Requirement: Float value form
A `float` value's kind SHALL be a decimal number, the percentage. Its CLI text SHALL be `float` followed by the percentage with two decimals and `%`.

#### Scenario: Reference float
- **WHEN** the field at (156,22)–(193,50) of `testdata/example1.png` is read
- **THEN** its value is the decimal number 36.842…, and its CLI text is `float 36.84%`

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
