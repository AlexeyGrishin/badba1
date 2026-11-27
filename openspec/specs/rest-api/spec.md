# rest-api Specification

## Purpose

Defines the badba1 web server's REST API: which databases it serves and how they are identified, how records and fields are represented as JSON and paged, how image data is fetched, how field values are written, and how errors are reported.

## Requirements

### Requirement: Server startup
The `server` binary SHALL serve the databases in a folder, `db` by default or the folder given with `--db <dir>`. It listens on `127.0.0.1` at port 8080 by default, or the port given with `--port <n>`. It exits with an error message if the folder does not exist or the port cannot be bound.

#### Scenario: Defaults
- **WHEN** `server` is started with no arguments in the repository root
- **THEN** it serves the PNG files in `db/` on `http://127.0.0.1:8080`

#### Scenario: Missing folder
- **WHEN** `server --db no-such-dir` is started
- **THEN** it prints an error naming `no-such-dir` and exits with a non-zero status

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

### Requirement: Records page
`GET /api/dbs/{db}/records?offset=O&limit=L` SHALL return the database's records in record order, skipping the first O and returning at most L. The response includes `total` (the number of records), `offset`, `limit` and `background` (the image's top-left pixel color as `#RRGGBB`). Offset defaults to 0, and limit defaults to 20 with a maximum of 1000.

#### Scenario: Page of records
- **WHEN** a client requests `/api/dbs/example3/records?offset=1&limit=1` for a copy of `testdata/example3.png`
- **THEN** the response has `total` 3, `offset` 1, `limit` 1, `background` `#FFFFFF`, and one record with id `"250/23"`

#### Scenario: Offset past the end
- **WHEN** offset is 10 for a database with 3 records
- **THEN** the response has `total` 3 and an empty `records` list

### Requirement: Field JSON
Each record SHALL list its fields in record order, each with `name` (`field_0`, `field_1`, …), `type`, and a type-specific value:
- `counter`: `value` is the count
- `float`: `value` is the percentage
- `bool`: `value` is `true` or `false`
- `string`: `value` is the text
- `ref`: `value` is the target record's id, or `null`
- `image`: no value, but `width` and `height` of the data area

#### Scenario: Record with several types
- **WHEN** the record `"35/21"` of the reference image is listed
- **THEN** its fields are `field_0` image 33×20, `field_1` counter 4, `field_2` float about 43.42, and `field_3` ref `"250/23"`

#### Scenario: String field
- **WHEN** the record `"250/23"` of the reference image is listed
- **THEN** its last field is a string with value `"OPENSPEC"`

### Requirement: Image data
`GET /api/dbs/{db}/records/{x}/{y}/fields/{i}/image` SHALL return the data area of image field `i` of that record as a PNG of the data area's size. Requesting it for a non-image field gives a 404 response.

#### Scenario: Image field pixels
- **WHEN** a client requests `/api/dbs/example3/records/35/21/fields/0/image`
- **THEN** the response is a 33×20 PNG, with content type `image/png`, equal to that field's data area

### Requirement: Write a field
`PUT /api/dbs/{db}/records/{x}/{y}/fields/{i}` with a JSON body `{"value": …}` SHALL write the value into field `i` of that record. The JSON type must suit the field: a number for counter and float, `true`/`false` for bool, a string for string. `PUT …/fields/{i}/image` with a PNG body writes an image field. A successful write saves the database file, and the response is the field's JSON with the stored value.

#### Scenario: Float stores what fits
- **WHEN** a client sends `{"value": 33.3}` to field `2` of record `"35/21"`
- **THEN** the response is that field's JSON with a value of about 33.27, and a later records request shows the same value

#### Scenario: Image upload
- **WHEN** a client sends a 10×10 PNG to `/records/35/21/fields/0/image`
- **THEN** the response is the image field's JSON, and the field's image endpoint returns a 33×20 PNG whose top-left 10×10 pixels equal the upload

#### Scenario: Write persists to the file
- **WHEN** a counter field is written with 3, and the database file is then analyzed with the `cli` tool
- **THEN** that field reads as `counter 3`

### Requirement: Error responses
Errors SHALL be JSON `{"error": <kind>, "message": <text>}`, with the database file left unchanged:
- 404 for an unknown database, record or field index
- 400 for a malformed body or a value of the wrong JSON type
- 422 for write rule violations (`FieldOverflow`, `OutOfRange`, `UnknownCharacter`, `FrameColor`, `NotWritable`, `TypeMismatch`)
- 500 with `"error": "ImageTooLarge"` when the database's image is larger than the maximum image size

#### Scenario: Overflow
- **WHEN** a client writes `{"value": "HELLO WORLD"}` to the 40×5 string field of record `"250/23"`
- **THEN** the response is 422 with `"error": "FieldOverflow"`, and the file is byte-for-byte unchanged

#### Scenario: Reference field
- **WHEN** a client writes any value to a ref field
- **THEN** the response is 422 with `"error": "NotWritable"`

#### Scenario: Unknown record
- **WHEN** a client writes to `/records/1/1/fields/0`
- **THEN** the response is 404 with a JSON error

#### Scenario: Wrong JSON type
- **WHEN** a client sends `{"value": "abc"}` to a counter field
- **THEN** the response is 400 with a JSON error

#### Scenario: Database image too large
- **WHEN** the db folder holds `huge.png`, a valid 20001×20000 PNG, and a client requests `/api/dbs/huge/records`
- **THEN** the response is 500 with `"error": "ImageTooLarge"` and a message that gives the image's size and the maximum

#### Scenario: Maximum-size database is served
- **WHEN** the db folder holds the 10000×40000 `big.png` from the `gen_big` example, and a client requests `/api/dbs/big/records`
- **THEN** the response is 200 with the first page of records

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

### Requirement: Writes update the in-memory database
A successful write SHALL update the database held in memory as well as its file, so later requests see the new value without reading the file. If the file cannot be saved, the write SHALL be undone in memory and the response is an error.

#### Scenario: Later reads see the write from memory
- **WHEN** a client writes `{"value": 4}` to a counter field, the file is then replaced on disk by a different valid database image, and the client requests the records page from the same server
- **THEN** the records page shows that field with value 4

#### Scenario: Failed save leaves memory unchanged
- **WHEN** a write is valid but the database file cannot be saved
- **THEN** the response is a 500 JSON error, and a later records request shows the field's previous value

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
