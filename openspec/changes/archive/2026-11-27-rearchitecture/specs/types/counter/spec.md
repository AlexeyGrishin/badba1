# Spec Delta

## Purpose

Defines the `counter` data type: its type mark, how its value is read as the number of separate objects in the data area, how a value is written as squares, and how the value is shown.

## ADDED Requirements

### Requirement: Counter mark
A field's type SHALL be `counter` when its mark shape is `###` / `#..` / `###` (rows top to bottom, `#` = mark color).

#### Scenario: Counter mark
- **WHEN** the mark area holds red (237,28,36) pixels in the shape `###` / `#..` / `###`, and white pixels elsewhere
- **THEN** the field's type is `counter` and its mark color is (237,28,36)

### Requirement: Counter value
The value of a `counter` field SHALL be the number of separate objects in its data area. An object is a group of mark-color pixels connected horizontally or vertically. Pixels that touch only diagonally belong to different objects. Pixels of every other color count as empty.

#### Scenario: Separate dots
- **WHEN** a `counter` field with a red mark has 3 separate red blobs in its data area
- **THEN** its value is 3

#### Scenario: Diagonal contact does not join
- **WHEN** two red pixels in a `counter` field's data area touch only at a corner
- **THEN** they count as 2 objects

#### Scenario: Other colors do not count
- **WHEN** a `counter` field with a red mark has 2 red blobs and 4 blue blobs in its data area
- **THEN** its value is 2

#### Scenario: Empty area
- **WHEN** a `counter` field's data area has no pixels of the mark color
- **THEN** its value is 0

### Requirement: Counter value form
A `counter` value's kind SHALL be a whole number. Its CLI text SHALL be `counter` followed by the number.

#### Scenario: Reference counter
- **WHEN** the field at (101,22)–(137,47) of `testdata/example1.png` is read
- **THEN** its value is the whole number 5, and its CLI text is `counter 5`

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
