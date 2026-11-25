# Spec Delta

## MODIFIED Requirements

### Requirement: Field listing
For every field, the output SHALL show the field's number, its outer rectangle as left, top, right and bottom pixel coordinates, its outer width and height, its border thickness on each side (top, right, bottom, left), its data-area rectangle and size, and its data type. For `float` fields it SHALL also show the value as a percentage with two decimals, for `counter` fields the count, and for `bool` fields `true` or `false`. `image` fields show no value. Fields are listed in number order.

#### Scenario: Field line content
- **WHEN** an image has a field with outer rectangle (101,22)–(137,47), borders of 5 on every side and data area (106,27)–(132,42)
- **THEN** its line shows its number, `(101,22)-(137,47)`, `37x26`, the four border values `5`, and `(106,27)-(132,42)` with `27x16`

#### Scenario: Counter field
- **WHEN** a field is a `counter` with value 5
- **THEN** its line ends with `counter 5`

#### Scenario: Float field
- **WHEN** a field is a `float` with 196 of 532 data-area pixels in the mark color
- **THEN** its line ends with `float 36.84%`

#### Scenario: Bool field
- **WHEN** a field is a `bool` with value true, and another is a `bool` with value false
- **THEN** their lines end with `bool true` and `bool false`, respectively

#### Scenario: Image field
- **WHEN** a field is an `image` with a 33×20 data area
- **THEN** its line shows `33x20` for the data area and ends with `image`, with no value
