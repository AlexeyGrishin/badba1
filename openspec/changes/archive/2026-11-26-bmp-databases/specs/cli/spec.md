# Spec Delta

## MODIFIED Requirements

### Requirement: Show layout of an image
Running `cli <path>` with the path of a PNG or BMP file SHALL print a summary line with the file path, image size, field count, link count and record count. It SHALL then list each field, each link and each record, and exit with status 0. The format is chosen by the file extension, `.png` or `.bmp` (in any letter case). Any other extension is read as PNG.

#### Scenario: Reference image
- **WHEN** a user runs `cli testdata/example1.png`
- **THEN** the output says the image is 539×348 and has 5 fields, 3 links and 2 records
- **AND** the command exits with status 0

#### Scenario: BMP image
- **WHEN** a user runs `cli example1.bmp` on `testdata/example1.png` saved as a 24-bit BMP
- **THEN** the output is the same as for `cli testdata/example1.png` apart from the file path, and the exit status is 0
