# Spec Delta

## Purpose

Defines the `bool` data type: its type mark, how its value is read from whether the data area holds any mark-color pixel, how a value is written, and how the value is shown.

## ADDED Requirements

### Requirement: Bool mark
A field's type SHALL be `bool` when its mark shape is `.#.` / `.##` / `.##` (rows top to bottom, `#` = mark color).

#### Scenario: Bool mark
- **WHEN** the mark area holds pink (255,174,201) pixels in the shape `.#.` / `.##` / `.##`, and white pixels elsewhere
- **THEN** the field's type is `bool` and its mark color is (255,174,201)

#### Scenario: Top-left pixel in another color
- **WHEN** the mark area holds a pink `bool` shape and its top-left pixel is black
- **THEN** the field's type is `bool` and its mark color is pink

### Requirement: Bool value
The value of a `bool` field SHALL be true when at least one pixel of its data area is exactly the mark color, and false when none is. Pixels of every other color count as empty.

#### Scenario: One pixel makes it true
- **WHEN** a `bool` field with a pink mark has exactly one pink pixel in its data area
- **THEN** its value is true

#### Scenario: Filled area
- **WHEN** a `bool` field's data area holds a pink drawing of many pixels
- **THEN** its value is true

#### Scenario: Other colors only
- **WHEN** a `bool` field with a pink mark has black and red pixels in its data area, but no pink ones
- **THEN** its value is false

#### Scenario: Empty area
- **WHEN** a `bool` field's data area is all white
- **THEN** its value is false

### Requirement: Bool value form
A `bool` value's kind SHALL be a boolean. Its CLI text SHALL be `bool true` or `bool false`.

#### Scenario: Reference bools
- **WHEN** the fields at (310,29)–(327,49) and (288,31)–(305,49) of `testdata/example2.png` are read
- **THEN** their values are true and false, and their CLI texts are `bool true` and `bool false`

### Requirement: Bool writes
Writing true to a `bool` field SHALL fill its whole data area with the mark color. Writing false clears it.

#### Scenario: True and false
- **WHEN** false is written to a `bool` field that is true, and then true is written to it
- **THEN** it reads as `bool false` after the first write and `bool true` after the second
