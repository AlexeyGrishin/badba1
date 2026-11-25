# cli Specification

## Purpose

The `cli` command-line tool lets a person inspect a badba1 database image from a terminal, starting with showing the fields and links the backend detects in it.

## Requirements

### Requirement: Show layout of an image
Running `cli <path>` with the path of a PNG file SHALL print a summary line with the file path, image size, field count and link count. It SHALL then list each field and each link, and exit with status 0.

#### Scenario: Reference image
- **WHEN** a user runs `cli testdata/example1.png`
- **THEN** the output says the image is 539×348 and has 5 fields and 3 links
- **AND** the command exits with status 0

### Requirement: Field listing
For every field, the output SHALL show the field's number, its outer rectangle as left, top, right and bottom pixel coordinates, its outer width and height, its border thickness on each side (top, right, bottom, left), its data-area rectangle and size, and its data type. For `float` fields it SHALL also show the value as a percentage with two decimals, and for `counter` fields the count. `image` fields show no value. Fields are listed in number order.

#### Scenario: Field line content
- **WHEN** an image has a field with outer rectangle (101,22)–(137,47), borders of 5 on every side and data area (106,27)–(132,42)
- **THEN** its line shows its number, `(101,22)-(137,47)`, `37x26`, the four border values `5`, and `(106,27)-(132,42)` with `27x16`

#### Scenario: Counter field
- **WHEN** a field is a `counter` with value 5
- **THEN** its line ends with `counter 5`

#### Scenario: Float field
- **WHEN** a field is a `float` with 196 of 532 data-area pixels in the mark color
- **THEN** its line ends with `float 36.84%`

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
When the file cannot be read or is not a valid PNG, `cli` SHALL print an error naming the file to standard error and exit with status 1.

#### Scenario: Missing file
- **WHEN** a user runs `cli does-not-exist.png`
- **THEN** an error mentioning `does-not-exist.png` is printed to standard error and the exit status is 1
