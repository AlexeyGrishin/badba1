# cli Specification

## Purpose

The `cli` command-line tool lets a person inspect a badba1 database image from a terminal, starting with showing the fields and links the backend detects in it.

## Requirements

### Requirement: Show layout of an image
Running `cli <path>` with the path of a PNG file SHALL print a summary line with the file path, image size, field count, link count and record count. It SHALL then list each field, each link and each record, and exit with status 0.

#### Scenario: Reference image
- **WHEN** a user runs `cli testdata/example1.png`
- **THEN** the output says the image is 539×348 and has 5 fields, 3 links and 2 records
- **AND** the command exits with status 0

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

### Requirement: Link listing
For every link, the output SHALL show the link's number and the numbers of all fields it connects.

#### Scenario: Link line content
- **WHEN** an image has one link connecting fields 1 and 3
- **THEN** the output has a link line showing link 1 connecting `F1` and `F3`

#### Scenario: No links
- **WHEN** an image has fields but no links
- **THEN** the summary reports 0 links and the output has no link lines

### Requirement: Usage errors
When run without exactly one path argument, `cli` SHALL print a usage message naming the expected argument to standard error and exit with status 2.

#### Scenario: No argument
- **WHEN** a user runs `cli` with no arguments
- **THEN** a usage message is printed to standard error and the exit status is 2

### Requirement: File errors
When the file cannot be read, is not a valid PNG, or is larger than the maximum image size, `cli` SHALL print an error naming the file to standard error and exit with status 1.

#### Scenario: Missing file
- **WHEN** a user runs `cli does-not-exist.png`
- **THEN** an error mentioning `does-not-exist.png` is printed to standard error and the exit status is 1

#### Scenario: Image too large
- **WHEN** a user runs `cli huge.png` on a valid 20001×20000 PNG
- **THEN** an error that mentions `huge.png`, says the image is too large and gives the 400,000,000-pixel maximum is printed to standard error, and the exit status is 1

#### Scenario: Maximum-size image
- **WHEN** a user runs `cli db/big.png` on the 10000×40000 image from the `gen_big` example
- **THEN** the layout is printed, starting `db/big.png: 10000x40000`, and the exit status is 0

### Requirement: Record listing
For every record, the output SHALL show the record's number and the numbers of its fields, in record order. Records are listed in number order, after the links.

#### Scenario: Record lines for the reference image
- **WHEN** a user runs `cli testdata/example1.png`
- **THEN** the output ends with a records section listing `R1  F1, F2, F3` and `R2  F4, F5`

#### Scenario: Lone field
- **WHEN** an image has a single field and no links
- **THEN** the output lists one record, `R1  F1`
