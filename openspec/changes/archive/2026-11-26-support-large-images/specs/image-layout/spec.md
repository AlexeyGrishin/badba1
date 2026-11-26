# Spec Delta

## ADDED Requirements

### Requirement: Maximum image size
The system SHALL read and analyze any valid PNG of up to 400,000,000 pixels (width × height), of any shape, color type or bit depth. A PNG with more pixels SHALL be rejected, with no layout, by an error that says the image is too large and gives its dimensions and the maximum. The rejection SHALL happen before the pixel data is decoded.

#### Scenario: Maximum-size image is analyzed
- **WHEN** the system is given a 10000×40000 PNG database image, such as `db/big.png` from the `gen_big` example
- **THEN** it returns the image's layout with width 10000 and height 40000, and every generated record is detected

#### Scenario: Other shape at the maximum
- **WHEN** the system is given a valid 20000×20000 PNG
- **THEN** it returns the image's layout

#### Scenario: One pixel over the maximum
- **WHEN** the system is given a valid PNG with 400,000,001 or more pixels, such as 20001×20000
- **THEN** it reports an error that says the image is too large, names the file, and gives `20001x20000` and the 400,000,000-pixel maximum. It returns no layout.

#### Scenario: Too-large image is rejected quickly
- **WHEN** the system is given a PNG whose header declares 100000×100000 pixels
- **THEN** it reports the too-large error without allocating memory for the pixels

#### Scenario: Small images are unaffected
- **WHEN** the system is given any image it could analyze before, such as `testdata/example1.png`
- **THEN** it returns the same layout as before
