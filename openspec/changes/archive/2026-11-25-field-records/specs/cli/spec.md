# Spec Delta

## MODIFIED Requirements

### Requirement: Show layout of an image
Running `cli <path>` with the path of a PNG file SHALL print a summary line with the file path, image size, field count, link count and record count. It SHALL then list each field, each link and each record, and exit with status 0.

#### Scenario: Reference image
- **WHEN** a user runs `cli testdata/example1.png`
- **THEN** the output says the image is 539×348 and has 5 fields, 3 links and 2 records
- **AND** the command exits with status 0

## ADDED Requirements

### Requirement: Record listing
For every record, the output SHALL show the record's number and the numbers of its fields, in record order. Records are listed in number order, after the links.

#### Scenario: Record lines for the reference image
- **WHEN** a user runs `cli testdata/example1.png`
- **THEN** the output ends with a records section listing `R1  F1, F2, F3` and `R2  F4, F5`

#### Scenario: Lone field
- **WHEN** an image has a single field and no links
- **THEN** the output lists one record, `R1  F1`
