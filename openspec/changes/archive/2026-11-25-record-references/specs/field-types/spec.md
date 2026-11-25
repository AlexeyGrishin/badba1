# Spec Delta

## MODIFIED Requirements

### Requirement: Mark color and shape
The mark color SHALL be the exact color of the top-middle pixel of the mark area. The mark's shape is the set of the 9 pixels that have exactly that color. A field's type SHALL be `float` when the shape is `###` / `##.` / `#..` (rows top to bottom, `#` = mark color). It SHALL be `counter` when the shape is `###` / `#..` / `###`, `bool` when the shape is `.#.` / `.##` / `.##`, and `ref` when the shape is `.#.` / `###` / `.#.`.

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

#### Scenario: Other pixels are not the mark color
- **WHEN** the mark area holds a green `float` shape and the remaining pixels are a mix of white and black
- **THEN** the field's type is `float`

#### Scenario: Top-left pixel in another color
- **WHEN** the mark area holds a pink `bool` shape and its top-left pixel is black
- **THEN** the field's type is `bool` and its mark color is pink

### Requirement: Default type is image
A field's type SHALL be `image` when its mark area has no pixels, holds one color only, holds a shape other than the `float`, `counter`, `bool` and `ref` shapes, or uses the frame color `#BADBA1` as its mark color. An `image` field has no value.

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

## ADDED Requirements

### Requirement: Ref data area is ignored
The data area of a `ref` field SHALL have no meaning: whatever is drawn in it does not affect the field's type or reference.

#### Scenario: Drawing inside a ref field
- **WHEN** a `ref` field with a yellow mark has yellow and black pixels in its data area
- **THEN** it is still a `ref` field, and its reference is the same as with an empty data area
