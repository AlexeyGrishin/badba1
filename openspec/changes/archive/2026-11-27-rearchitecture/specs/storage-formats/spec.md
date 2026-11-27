# Spec Delta

## Purpose

Defines the storage-format framework: how the format of a database file is chosen, the size limit every format enforces, and what each format, kept in its own spec and module under `formats/`, must provide so that new formats can be added without touching the rest of the system.

## ADDED Requirements

### Requirement: Registered formats
The library SHALL keep a list of registered storage formats, currently `png` and `bmp`. Each format SHALL have a name and the file extensions it owns. Every part of the system that needs to know the formats (loading, CLI, REST database listing) SHALL take them from this list.

#### Scenario: Formats and extensions
- **WHEN** a program asks the library for its registered formats
- **THEN** it gets `png` owning `.png` and `bmp` owning `.bmp`

### Requirement: Format chosen by extension
A database file's format SHALL be the registered format that owns its extension, compared in any letter case. A file whose extension no format owns SHALL be read as PNG.

#### Scenario: Upper-case extension
- **WHEN** `load_db` is given `EXAMPLE1.BMP`
- **THEN** the file is read as BMP

#### Scenario: Unknown extension
- **WHEN** `load_db` is given a valid PNG named `example1.img`
- **THEN** the file is read as PNG and its layout is returned

### Requirement: Maximum image size
The system SHALL read and analyze any valid image of up to 400,000,000 pixels (width × height), of any shape, in every registered format. An image with more pixels SHALL be rejected, with no layout, by an error that says the image is too large and gives its dimensions and the maximum. The rejection SHALL happen before the pixel data is decoded.

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

### Requirement: Same layout in every format
Every registered format SHALL give the same layout, field values and field images for the same pixels. Detection, types and records SHALL NOT depend on the format.

#### Scenario: PNG and BMP agree
- **WHEN** `testdata/example1.png` is saved in every registered format and each file is loaded
- **THEN** each gives 539×348, 5 fields, 3 links and 2 records with identical rectangles and values

### Requirement: Format contract
Each format SHALL provide: whether it owns a file, reading the image size from the header, reading the whole image as RGBA pixels, reading a field's data-area pixels, and storing a written data area back into the file so that a failed store leaves the file and the in-memory database as they were. How pixels are kept between reads is up to the format.

#### Scenario: Written data area is stored
- **WHEN** a field of a database is written through the library, in any registered format
- **THEN** loading the file again gives the new value, and every other field's value and pixels are unchanged

### Requirement: One module and one spec per format
Each storage format SHALL be defined in its own spec, `formats/<name>`, and implemented in its own module under the library's `formats` folder. Adding a format SHALL need only a new spec, a new module, and one entry in the format list; no other module of the library, the CLI or the server SHALL name a specific format.

#### Scenario: Format knowledge stays in its module
- **WHEN** the library, CLI and server sources are searched for format-specific code (BMP header handling, reading or saving PNG files, `.png`/`.bmp` checks)
- **THEN** it is found only in that format's module, the format list, and tests; the REST API's PNG encoding of field images for HTTP responses is not storage and may stay in the server
