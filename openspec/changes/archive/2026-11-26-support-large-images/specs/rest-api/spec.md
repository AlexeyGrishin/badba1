# Spec Delta

## MODIFIED Requirements

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
