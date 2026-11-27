# formats/bmp Specification

## Purpose

Defines the BMP storage format: which BMP variants can be read, and how a BMP database keeps its pixels on disk and is written in place, one data area at a time.

## Requirements

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

### Requirement: BMP databases keep pixels on disk
For a BMP database, the library SHALL keep only the layout, field values and background color in memory after loading. A field's image SHALL be read from the file's rows for that data area. A write SHALL overwrite only that data area's bytes in place, leaving the file's size and all other bytes unchanged. Results SHALL be the same as for the same database stored as PNG.

#### Scenario: Same results as PNG
- **WHEN** `example3.bmp` is `testdata/example3.png` saved as a 24-bit BMP, and a program reads its records and the image of field 0 of record `35/21`
- **THEN** both equal the results for `example3.png`

#### Scenario: Write patches the file in place
- **WHEN** a program writes 5 to the counter field 1 of record `35/21` in `example3.bmp`
- **THEN** the write returns 5, the file has the same size, only bytes within that field's data-area rows differ, and `cli example3.bmp` shows that field as `counter 5`

#### Scenario: Image written into a BMP
- **WHEN** a program writes a 10×10 bitmap with partly transparent pixels to field 0 of record `35/21` of a 24-bit `example3.bmp`
- **THEN** the field's image is a 33×20 bitmap whose top-left 10×10 pixels equal the bitmap with alpha made fully opaque

#### Scenario: Failed in-place write
- **WHEN** a write to a BMP database is valid but the file cannot be written
- **THEN** the write fails with a save error, the data area's previous bytes are restored where possible, and the field's value and pixels read from the database are those from before the write

#### Scenario: Large BMP stays small in memory
- **WHEN** a 10000×40000 `big.bmp` is loaded and its records have been read once
- **THEN** the process's memory use stays below 500 MB, while reading the records and a field image each take under 100 ms
