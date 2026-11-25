# web-ui Specification

## Purpose

Defines the browser page the badba1 server provides for viewing a database's records and editing their field values, rendered in the record layout of the user's mockup.

## Requirements

### Requirement: Page and navigation
The server SHALL serve a web page at `/` that lists the available databases, shows the records of the selected one a page at a time, and offers previous/next paging over the records API.

#### Scenario: Open the page
- **WHEN** a user opens `http://127.0.0.1:8080/` with databases in the folder
- **THEN** the page shows a database selector and the first page of records of the first database

#### Scenario: Paging
- **WHEN** the user goes to the next page
- **THEN** the page shows the following records, using the API's offset and limit

### Requirement: Record layout
Each record SHALL be shown as in the mockup: an opening brace, `id: "x/y",`, one line per field `field_i: <value or editor>,` in record order, and a closing brace. A ref field is shown read-only as `&"x/y"`, or `&null` for a null reference.

#### Scenario: Record rendering
- **WHEN** the record `"35/21"` of the reference image is shown
- **THEN** it reads `{`, `id: "35/21",`, then `field_0:` with its image, `field_1:` with the counter, `field_2:` with the float and `%`, `field_3: &"250/23",`, and `}`

### Requirement: Editing scalar fields
Counter, float, bool and string fields SHALL be editable in place: a number input (counter), a number input followed by `%` (float), a checkbox (bool), and a text input (string). A change is sent to the write API right away. The editor then shows the value the API returns. On an error, the editor shows the error message and returns to the last stored value.

#### Scenario: Float shows the stored value
- **WHEN** the user enters 33.3 in a float field of a 532-pixel data area
- **THEN** the field shows 33.27 after saving

#### Scenario: Rejected value
- **WHEN** the user enters text that does not fit in a string field
- **THEN** the field shows the FieldOverflow message and its previous text

### Requirement: Drawing in image fields
Image fields SHALL be shown as a zoomed canvas of the data area. Holding the left mouse button draws black pixels, and holding the right mouse button erases pixels to the database's background color (without opening the browser's context menu). When the button is released, the whole data area is sent to the image write API.

#### Scenario: Draw and save
- **WHEN** the user draws a line with the left mouse button in an image field and releases it
- **THEN** the drawn pixels are black, the image is saved, and reloading the page shows the line

#### Scenario: Erase
- **WHEN** the user drags with the right mouse button over black pixels
- **THEN** those pixels become the background color and are saved, and no context menu appears
