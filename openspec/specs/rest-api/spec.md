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
Each `.png` file directly in the database folder SHALL be a database whose id is the file name without `.png`. Ids SHALL consist only of letters, digits, `-` and `_`. A request naming any other id, or an id with no matching file, gets a 404 response.

#### Scenario: List databases
- **WHEN** the folder holds `example1.png`, `b.png` and `notes.txt`, and a client sends `GET /api/dbs`
- **THEN** the response is `{"dbs": ["b", "example1"]}`, with ids sorted

#### Scenario: Unknown or unsafe id
- **WHEN** a client requests `/api/dbs/missing/records` or `/api/dbs/..%2Fsecret/records`
- **THEN** the response is 404 with a JSON error

### Requirement: Record ids
A record's id SHALL be `"x/y"`, where (x, y) is the top-left corner of its first field's frame. In URLs a record is addressed as `/records/{x}/{y}`.

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
