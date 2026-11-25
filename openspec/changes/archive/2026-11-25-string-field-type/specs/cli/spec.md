# Spec Delta

## MODIFIED Requirements

### Requirement: Field listing
For every field, in number order, the output SHALL show its number, outer rectangle (left, top, right, bottom) with width and height, border thickness per side, data-area rectangle and size, and data type. Typed fields add their value: `float` as a percentage with two decimals, `counter` as the count, `bool` as `true`/`false`, `ref` as `R<number>` or `null`, `string` as quoted, escaped text. `image` fields have no value.

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

#### Scenario: Ref field
- **WHEN** a field is a `ref` whose target is record 2, and another is a `ref` with a null reference
- **THEN** their lines end with `ref R2` and `ref null`, respectively

#### Scenario: String field
- **WHEN** a field is a `string` with text `OPENSPEC`, and another has the two lines `AB` and `CD`
- **THEN** their lines end with `string "OPENSPEC"` and `string "AB\nCD"`, respectively

#### Scenario: Image field
- **WHEN** a field is an `image` with a 33×20 data area
- **THEN** its line shows `33x20` for the data area and ends with `image`, with no value
