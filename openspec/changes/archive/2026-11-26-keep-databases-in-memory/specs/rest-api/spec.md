# Spec Delta

## ADDED Requirements

### Requirement: Databases held in memory
The server SHALL read and analyze a database's file at most once per run: when a request first names that database. Later requests SHALL be served from memory. Changes made to the file by other programs while the server runs SHALL NOT be seen until restart. A load that fails SHALL NOT be remembered, so a later request tries again.

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
