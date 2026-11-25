# Spec Delta

## MODIFIED Requirements

### Requirement: Mark color and shape
The mark color SHALL be the exact color of the top-middle pixel of the mark area. The mark's shape is the set of the 9 pixels that have exactly that color. A field's type SHALL be `float` when the shape is `###` / `##.` / `#..` (rows top to bottom, `#` = mark color). It SHALL be `counter` when the shape is `###` / `#..` / `###`, and `bool` when the shape is `.#.` / `.##` / `.##`.

#### Scenario: Float mark
- **WHEN** the mark area holds green (34,177,76) pixels in the shape `###` / `##.` / `#..`, and white pixels elsewhere
- **THEN** the field's type is `float` and its mark color is (34,177,76)

#### Scenario: Counter mark
- **WHEN** the mark area holds red (237,28,36) pixels in the shape `###` / `#..` / `###`, and white pixels elsewhere
- **THEN** the field's type is `counter` and its mark color is (237,28,36)

#### Scenario: Bool mark
- **WHEN** the mark area holds pink (255,174,201) pixels in the shape `.#.` / `.##` / `.##`, and white pixels elsewhere
- **THEN** the field's type is `bool` and its mark color is (255,174,201)

#### Scenario: Other pixels are not the mark color
- **WHEN** the mark area holds a green `float` shape and the remaining pixels are a mix of white and black
- **THEN** the field's type is `float`

#### Scenario: Top-left pixel in another color
- **WHEN** the mark area holds a pink `bool` shape and its top-left pixel is black
- **THEN** the field's type is `bool` and its mark color is pink

### Requirement: Default type is image
A field's type SHALL be `image` when its mark area has no pixels, holds one color only, holds a shape other than the `float`, `counter` and `bool` shapes, or uses the frame color `#BADBA1` as its mark color. An `image` field has no value.

#### Scenario: No mark
- **WHEN** the mark area is all white
- **THEN** the field's type is `image`

#### Scenario: Unrecognized shape
- **WHEN** the mark area holds green pixels in the shape `#.#` / `.#.` / `#.#`
- **THEN** the field's type is `image`

#### Scenario: Plus-shaped mark
- **WHEN** the mark area holds yellow pixels in the shape `.#.` / `###` / `.#.`
- **THEN** the field's type is `image`

#### Scenario: Frame-colored mark
- **WHEN** the mark area holds the `float` shape in the frame color
- **THEN** the field's type is `image`

## ADDED Requirements

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
