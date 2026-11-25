# field-types Specification

## Purpose

Defines each field's data type, read from a small 3×3 mark drawn next to the field, and how the value of a typed field is computed from what is drawn in its data area.

## Requirements

### Requirement: Type mark location
A field's type mark SHALL be read from the 3×3 pixel area directly above the top-right corner of the field's outer rectangle. Its columns are `outer.right − 2` to `outer.right`, and its rows are `outer.top − 3` to `outer.top − 1`.

#### Scenario: Mark position for a field
- **WHEN** a field's outer rectangle is (156,22)–(193,50)
- **THEN** its mark area is columns 191 to 193 and rows 19 to 21

#### Scenario: No room above the field
- **WHEN** a field's outer top edge is at row 0, 1 or 2
- **THEN** the field has no mark area and its type is `image`

### Requirement: Mark color and shape
The mark color SHALL be the exact color of the top-middle pixel of the mark area. The mark's shape is the set of the 9 pixels that have exactly that color. A field's type SHALL be `float` when the shape is `###` / `##.` / `#..` (rows top to bottom, `#` = mark color), `counter` for `###` / `#..` / `###`, `bool` for `.#.` / `.##` / `.##`, `ref` for `.#.` / `###` / `.#.`, and `string` for `.#.` / `#.#` / `#.#`.

#### Scenario: Float mark
- **WHEN** the mark area holds green (34,177,76) pixels in the shape `###` / `##.` / `#..`, and white pixels elsewhere
- **THEN** the field's type is `float` and its mark color is (34,177,76)

#### Scenario: Counter mark
- **WHEN** the mark area holds red (237,28,36) pixels in the shape `###` / `#..` / `###`, and white pixels elsewhere
- **THEN** the field's type is `counter` and its mark color is (237,28,36)

#### Scenario: Bool mark
- **WHEN** the mark area holds pink (255,174,201) pixels in the shape `.#.` / `.##` / `.##`, and white pixels elsewhere
- **THEN** the field's type is `bool` and its mark color is (255,174,201)

#### Scenario: Ref mark
- **WHEN** the mark area holds yellow (255,201,14) pixels in the shape `.#.` / `###` / `.#.`, and white pixels elsewhere
- **THEN** the field's type is `ref` and its mark color is (255,201,14)

#### Scenario: String mark
- **WHEN** the mark area holds black (0,0,0) pixels in the shape `.#.` / `#.#` / `#.#`, and white pixels elsewhere
- **THEN** the field's type is `string` and its mark color is (0,0,0)

#### Scenario: Other pixels are not the mark color
- **WHEN** the mark area holds a green `float` shape and the remaining pixels are a mix of white and black
- **THEN** the field's type is `float`

#### Scenario: Top-left pixel in another color
- **WHEN** the mark area holds a pink `bool` shape and its top-left pixel is black
- **THEN** the field's type is `bool` and its mark color is pink

### Requirement: Default type is image
A field's type SHALL be `image` when its mark area has no pixels, holds one color only, holds a shape other than the `float`, `counter`, `bool`, `ref` and `string` shapes, or uses the frame color `#BADBA1` as its mark color. An `image` field has no value.

#### Scenario: No mark
- **WHEN** the mark area is all white
- **THEN** the field's type is `image`

#### Scenario: Unrecognized shape
- **WHEN** the mark area holds green pixels in the shape `#.#` / `.#.` / `#.#`
- **THEN** the field's type is `image`

#### Scenario: Plus-shaped mark
- **WHEN** the mark area holds yellow pixels in the shape `.#.` / `###` / `.#.`
- **THEN** the field's type is `ref`, not `image`, because the plus shape is now a recognized shape

#### Scenario: Frame-colored mark
- **WHEN** the mark area holds the `float` shape in the frame color
- **THEN** the field's type is `image`

### Requirement: Float value
The value of a `float` field SHALL be the percentage of the data area's pixels that are exactly the mark color, from 0 to 100. Pixels of every other color count as empty.

#### Scenario: Partly filled area
- **WHEN** a `float` field with a green mark has a 10×10 data area in which 25 pixels are green, 10 are black and the rest are white
- **THEN** its value is 25%

#### Scenario: Empty and full areas
- **WHEN** a `float` field's data area has no pixels of the mark color, or only pixels of the mark color
- **THEN** its value is 0% or 100%, respectively

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

### Requirement: Reference image types
The fields of `testdata/example1.png` SHALL have these types and values, and SHALL keep them as the detection evolves.

#### Scenario: example1.png types and values
- **WHEN** `testdata/example1.png` is analyzed
- **THEN** the field with outer rectangle (101,22)–(137,47) is a `counter` with value 5
- **AND** the field at (156,22)–(193,50) is a `float` with 196 of its 532 data-area pixels in the mark color, a value of about 36.84%
- **AND** the field at (130,69)–(167,97) is a `counter` with value 3
- **AND** the fields at (35,23)–(77,52) and (72,70)–(107,97) are `image`

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

### Requirement: Second reference image types
The bool fields of `testdata/example2.png` SHALL have these values, and SHALL keep them as the detection evolves.

#### Scenario: example2.png bool fields
- **WHEN** `testdata/example2.png` (539×348) is analyzed
- **THEN** the field with outer rectangle (310,29)–(327,49) is a `bool` with value true
- **AND** the field at (288,31)–(305,49) is a `bool` with value false

### Requirement: Ref data area is ignored
The data area of a `ref` field SHALL have no meaning: whatever is drawn in it does not affect the field's type or reference.

#### Scenario: Drawing inside a ref field
- **WHEN** a `ref` field with a yellow mark has yellow and black pixels in its data area
- **THEN** it is still a `ref` field, and its reference is the same as with an empty data area
