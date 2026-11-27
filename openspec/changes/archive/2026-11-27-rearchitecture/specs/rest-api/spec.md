# Spec Delta

## ADDED Requirements

### Requirement: BMP databases served like PNG
The server SHALL serve a BMP database through the library's BMP storage format, with responses the same as for the same database stored as PNG.

#### Scenario: Same responses as PNG
- **WHEN** the folder holds `example3.bmp`, which is `testdata/example3.png` saved as a 24-bit BMP, and a client requests its records page and the image of field 0 of record `35/21`
- **THEN** both responses equal those for `example3.png`

#### Scenario: Write to a BMP over HTTP
- **WHEN** a client writes `{"value": 5}` to the counter field 1 of record `35/21` in `example3.bmp`
- **THEN** the response is 200 with value 5, the file has the same size, and `cli example3.bmp` shows that field as `counter 5`

#### Scenario: Image upload into a BMP
- **WHEN** a client sends a 10×10 PNG to field 0 of record `35/21` of `example3.bmp`
- **THEN** the field's image endpoint returns a 33×20 PNG whose top-left 10×10 pixels equal the upload, with the PNG's alpha made fully opaque

#### Scenario: Failed in-place write
- **WHEN** a write to a BMP database is valid but the file cannot be written
- **THEN** the response is a 500 JSON error, and later requests show the field's previous value and pixels

## MODIFIED Requirements

### Requirement: Database ids
Each file directly in the database folder whose extension is owned by a registered storage format (currently `.png` and `.bmp`) SHALL be a database whose id is the file name without its extension. Ids SHALL consist only of letters, digits, `-` and `_`. A request naming any other id, or an id with no matching file, gets a 404 response. An id with files in two or more formats is listed once, and every request to it gets a 409 response with `"error": "AmbiguousDatabase"`.

#### Scenario: List databases
- **WHEN** the folder holds `example1.png`, `b.png` and `notes.txt`, and a client sends `GET /api/dbs`
- **THEN** the response is `{"dbs": ["b", "example1"]}`, with ids sorted

#### Scenario: Unknown or unsafe id
- **WHEN** a client requests `/api/dbs/missing/records` or `/api/dbs/..%2Fsecret/records`
- **THEN** the response is 404 with a JSON error

#### Scenario: BMP databases are listed
- **WHEN** the folder holds `a.png` and `c.bmp`, and a client sends `GET /api/dbs`
- **THEN** the response is `{"dbs": ["a", "c"]}`

#### Scenario: Same id in both formats
- **WHEN** the folder holds `x.png` and `x.bmp`
- **THEN** `GET /api/dbs` lists `x` once, and `/api/dbs/x/records` returns 409 with `"error": "AmbiguousDatabase"`

### Requirement: Record ids
A record's id SHALL be the library's record id (see `library-api`): `"x/y"`, where (x, y) is the top-left corner of its first field's frame. In URLs a record is addressed as `/records/{x}/{y}`.

#### Scenario: Ids for the reference image
- **WHEN** the records of a database copied from `testdata/example3.png` are listed
- **THEN** their ids are `"35/21"`, `"250/23"` and `"35/68"`, in that order

## REMOVED Requirements

### Requirement: BMP databases keep pixels on disk
**Reason**: How a BMP database keeps its pixels and is written in place is a property of the BMP storage format, now in the library, so it moved to the `formats/bmp` capability.
**Migration**: See `formats/bmp`, requirement "BMP databases keep pixels on disk". REST responses for BMP databases stay the same as for PNG (covered there and by `storage-formats`, "Same layout in every format").
