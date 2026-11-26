# Spec Delta

## MODIFIED Requirements

### Requirement: Database ids
Each `.png` or `.bmp` file directly in the database folder SHALL be a database whose id is the file name without its extension. Ids SHALL consist only of letters, digits, `-` and `_`. A request naming any other id, or an id with no matching file, gets a 404 response. An id with both a `.png` and a `.bmp` file is listed once, and every request to it gets a 409 response with `"error": "AmbiguousDatabase"`.

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

### Requirement: Databases held in memory
The server SHALL read and analyze a database's file at most once per run: when a request first names that database. Later requests SHALL be served from memory. For PNG databases, changes made to the file by other programs while the server runs SHALL NOT be seen until restart. A load that fails SHALL NOT be remembered, so a later request tries again.

#### Scenario: Second request does not read the file
- **WHEN** a client requests `/api/dbs/example3/records`, the file `example3.png` is then replaced on disk by a different valid database image, and the client requests `/api/dbs/example3/records` again from the same server
- **THEN** both responses are identical and describe the original image

#### Scenario: Image data comes from memory
- **WHEN** a client has requested `/api/dbs/example3/records`, `example3.png` is then deleted from disk, and the client requests `/api/dbs/example3/records/35/21/fields/0/image`
- **THEN** the response is the 33×20 PNG of that field's data area from the original image

#### Scenario: Failed load is retried
- **WHEN** `bad.png` in the db folder is not a valid PNG, a client requests `/api/dbs/bad/records` and gets an error, `bad.png` is then replaced with a valid database image, and the client requests it again
- **THEN** the second response is 200 with that image's records

#### Scenario: New file is listed and served
- **WHEN** the server is running and a new valid database file `new.png` is copied into the db folder
- **THEN** `GET /api/dbs` lists `new`, and `/api/dbs/new/records` returns its records

## ADDED Requirements

### Requirement: BMP databases keep pixels on disk
For a BMP database, the server SHALL keep only the layout, field values and background color in memory after its first use. A field's image SHALL be read from the file's rows for that data area. A write SHALL overwrite only that data area's bytes in place, leaving the file's size and all other bytes unchanged. Responses SHALL be the same as for the same database stored as PNG.

#### Scenario: Same responses as PNG
- **WHEN** the folder holds `example3.bmp`, which is `testdata/example3.png` saved as a 24-bit BMP, and a client requests its records page and the image of field 0 of record `35/21`
- **THEN** both responses equal those for `example3.png`

#### Scenario: Write patches the file in place
- **WHEN** a client writes `{"value": 5}` to the counter field 1 of record `35/21` in `example3.bmp`
- **THEN** the response is 200 with value 5, the file has the same size, only bytes within that field's data-area rows differ, and `cli example3.bmp` shows that field as `counter 5`

#### Scenario: Image upload into a BMP
- **WHEN** a client sends a 10×10 PNG to field 0 of record `35/21` of `example3.bmp`
- **THEN** the field's image endpoint returns a 33×20 PNG whose top-left 10×10 pixels equal the upload, with the PNG's alpha made fully opaque

#### Scenario: Failed in-place write
- **WHEN** a write to a BMP database is valid but the file cannot be written
- **THEN** the response is a 500 JSON error, and later requests show the field's previous value and pixels

#### Scenario: Large BMP stays small in memory
- **WHEN** a 10000×40000 `big.bmp` is served and its records page has been requested once
- **THEN** the server process's memory use stays below 500 MB, while a records page and a field image each take under 100 ms
