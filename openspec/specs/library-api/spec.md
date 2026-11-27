# library-api Specification

## Purpose

Defines the `backend` library's single, documented entry point for embedding badba1 in other programs: loading a database, reading its records, fields, values and images, and writing field values, with the same results the CLI and REST API give.

## Requirements

### Requirement: Single entry point
The library SHALL offer one documented entry point for working with a database: `load_db` opens a database file and returns a database handle, and every database operation (records, fields, values, images, writes) is a method of that handle. Programs SHALL NOT need detection, type, format or file functions to use a database.

#### Scenario: Read a database through the entry point
- **WHEN** a program calls `load_db` with `testdata/example1.png` and asks the handle for its records
- **THEN** it gets 2 records whose fields, types and values equal what `cli testdata/example1.png` prints

#### Scenario: Same handle for every format
- **WHEN** a program calls `load_db` with `example1.bmp`, which is `testdata/example1.png` saved as a 24-bit BMP
- **THEN** it gets a handle with the same operations and the same results as for the PNG

### Requirement: Load errors
`load_db` SHALL fail, returning no handle, with an error that names the file when the file cannot be read, cannot be decoded in its format, uses an unsupported variant of its format, or is larger than the maximum image size. The error SHALL say which of these happened, so a client can tell them apart without parsing the message.

#### Scenario: Missing file
- **WHEN** a program calls `load_db` with a path that does not exist
- **THEN** it gets a read error naming the path

#### Scenario: Too large
- **WHEN** a program calls `load_db` with a PNG whose header declares 20001×20000 pixels
- **THEN** it gets a too-large error giving `20001x20000` and the 400,000,000-pixel maximum

### Requirement: Database properties
A database handle SHALL give the image's width and height, its background color (the image's top-left pixel), and the detected fields, links and records with their numbers.

#### Scenario: Reference image properties
- **WHEN** a program loads `testdata/example1.png`
- **THEN** the handle reports 539×348, the image's top-left color as background, 5 fields, 3 links and 2 records

### Requirement: Record ids
A record's id SHALL be `"x/y"`, where (x, y) is the top-left corner of its first field's frame. The handle SHALL list records in record order with their ids, and find a record by its id.

#### Scenario: Ids for the reference image
- **WHEN** a program loads `testdata/example3.png` and lists its records
- **THEN** their ids are `"35/21"`, `"250/23"` and `"35/68"`, in that order

#### Scenario: Unknown record id
- **WHEN** a program asks for record `"1/1"` in a database with no record at that id
- **THEN** it gets a not-found error naming the record

### Requirement: Get a field
The handle SHALL return field `i` (counting from 0 in record order) of a record, with its type name, its outer and data-area rectangles, and its value. A field index past the record's last field SHALL give a not-found error naming the record and index.

#### Scenario: Counter field
- **WHEN** a program asks `testdata/example1.png` for the field whose outer rectangle is (101,22)–(137,47)
- **THEN** it gets type `counter` and value 5

#### Scenario: Index past the end
- **WHEN** a program asks for field 9 of a record with 3 fields
- **THEN** it gets a not-found error naming the record and field 9

### Requirement: Generic field values
A field's value SHALL be one of a fixed set of value kinds that does not grow with the number of types: no value, whole number, decimal number, boolean, text, record reference (a record number, or null; the handle gives the record's id) and bitmap size. Each type SHALL also give its value as the text the CLI shows. Clients SHALL be able to display and convert every type's value using only the value kinds.

#### Scenario: Values of several types
- **WHEN** a program reads the fields of `testdata/example1.png` and `testdata/example2.png`
- **THEN** a counter's value is a whole number, a float's is a decimal percentage, a bool's is a boolean, a ref's is a record id or null, and an image field has no value but a bitmap size

#### Scenario: CLI text of a value
- **WHEN** a program asks for the CLI text of the float field at (156,22)–(193,50) of `testdata/example1.png`
- **THEN** it gets `float 36.84%`

### Requirement: Field image
The handle SHALL return the pixels of a field's data area as an RGBA bitmap of the data area's size, taken from the database as currently held.

#### Scenario: Image field pixels
- **WHEN** a program asks `testdata/example3.png` for the image of field 0 of record `35/21`
- **THEN** it gets a 33×20 bitmap equal to that field's data area

### Requirement: Write a field through the handle
The handle SHALL write a value into field `i` of a record, update the field's value in memory, save the change to the database file using the file's format, and return the value read back from the stored pixels. Field writes follow the field-writes rules and the field's type.

#### Scenario: Write and read back
- **WHEN** a program writes 4 to a counter field and then reads the field from the same handle
- **THEN** the write returns 4, the field's value is 4, and loading the file again with `load_db` also gives 4

#### Scenario: Write fails
- **WHEN** a program writes 113 to a counter field with a 27×16 data area
- **THEN** it gets a FieldOverflow error, and both the handle and the file still hold the previous value

### Requirement: Database held in memory
After `load_db`, the handle SHALL answer reads of records, fields and values from memory, and SHALL NOT see changes made to the file by other programs. Field images are also served from memory where the format keeps pixels in memory (PNG); a format that keeps pixels on disk (BMP) reads them from the file. A database file is read and analyzed once per `load_db` call.

#### Scenario: File replaced after loading
- **WHEN** a program loads `example3.png`, the file is then replaced by a different valid database image, and the program lists records from the same handle
- **THEN** the records describe the original image

### Requirement: Failed save rolls back
If a valid write cannot be saved to the file, the handle SHALL undo the write in memory and return a save error, so the handle and the file keep agreeing.

#### Scenario: File cannot be written
- **WHEN** a valid write is made but the database file cannot be saved
- **THEN** the program gets a save error, and the field's value and pixels read from the handle are those from before the write

### Requirement: Documented public API
Every public item of the library SHALL have API documentation. The crate documentation SHALL describe the entry point and include an example that loads a database, reads a field and writes a field; the example SHALL be compiled and run by `cargo test`.

#### Scenario: Documentation builds without gaps
- **WHEN** `cargo doc -p backend` is run
- **THEN** it succeeds with no missing-documentation warnings

#### Scenario: Crate example runs
- **WHEN** `cargo test -p backend --doc` is run
- **THEN** the crate-level example compiles and passes
