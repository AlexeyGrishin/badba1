# Spec Delta

## Purpose

Defines the PNG storage format: which PNG files can be read, how a PNG database's pixels are held once loaded, and how a write is saved back to the file.

## ADDED Requirements

### Requirement: Read PNG images
The system SHALL accept a PNG image in any standard PNG color type and bit depth, given as a file path. It SHALL report an error, and no layout, when the file cannot be read or is not a valid PNG.

#### Scenario: Valid PNG is read
- **WHEN** the system is given the path of a valid PNG file, such as `testdata/example1.png`
- **THEN** it returns the image's layout, along with the image's width and height in pixels

#### Scenario: Missing file
- **WHEN** the system is given a path that does not exist
- **THEN** it reports an error naming the path, and returns no layout

#### Scenario: Not a PNG
- **WHEN** the system is given a file that is not a valid PNG, such as a text file
- **THEN** it reports an error saying the file could not be decoded as PNG, and returns no layout

### Requirement: PNG pixels held in memory
A loaded PNG database SHALL keep its decoded pixels in memory. Field values and field images SHALL be served from memory without reading the file again, even if the file is changed or deleted.

#### Scenario: Image after the file is deleted
- **WHEN** a program loads `example3.png`, the file is then deleted, and the program asks for the image of field 0 of record `35/21`
- **THEN** it gets the 33×20 data area from the original image

### Requirement: PNG save through a temporary file
A write to a PNG database SHALL save the whole image as an 8-bit RGBA PNG to a temporary file in the same folder and then rename it over the database file, so the file is never left half-written. If saving fails, the in-memory pixels of the data area SHALL be put back and the write SHALL fail with a save error.

#### Scenario: Write persists to the file
- **WHEN** a value is written to a field of a PNG database
- **THEN** loading the file again gives the new value, and no temporary file is left in the folder

#### Scenario: Save fails
- **WHEN** a valid write is made to a PNG database whose file cannot be replaced
- **THEN** the write fails with a save error, and the database's pixels and value for that field are those from before the write
