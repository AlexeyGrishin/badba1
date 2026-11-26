# Spec Delta

## ADDED Requirements

### Requirement: Read BMP images
The system SHALL accept an uncompressed BMP file, given as a file path, with 24-bit pixels or 32-bit pixels in standard byte order, stored bottom-up or top-down. It SHALL return the same layout as for the same pixels stored as PNG. 24-bit pixels and 32-bit pixels without an alpha mask count as fully opaque. Other BMP variants SHALL be rejected with an error naming the file and the unsupported format.

#### Scenario: 24-bit BMP matches the PNG
- **WHEN** `testdata/example1.png` is saved as a 24-bit bottom-up BMP and the system is given that BMP
- **THEN** it returns the same layout as for `testdata/example1.png`: 539×348, 5 fields, 3 links and 2 records with identical rectangles and values

#### Scenario: Top-down and 32-bit BMPs
- **WHEN** the same pixels are stored as a top-down 24-bit BMP, and as a 32-bit BMP
- **THEN** each returns the same layout as the PNG

#### Scenario: Unsupported BMP
- **WHEN** the system is given an RLE-compressed or 8-bit palette BMP
- **THEN** it reports an error that names the file and says the BMP format is not supported, and returns no layout

#### Scenario: BMP over the maximum size
- **WHEN** the system is given a BMP whose header declares 20001×20000 pixels
- **THEN** it reports the too-large error from the header, without reading the pixel data

#### Scenario: Not a BMP
- **WHEN** the system is given a `.bmp` file that is not a BMP, such as a text file
- **THEN** it reports an error saying the file could not be decoded as BMP, and returns no layout
