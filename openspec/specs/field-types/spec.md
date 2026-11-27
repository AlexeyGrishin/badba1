# field-types Specification

## Purpose

Defines each field's data type, read from a small 3×3 mark drawn next to the field, and how the value of a typed field is computed from what is drawn in its data area.

## Requirements

### Requirement: Type mark location
A field's type mark SHALL be read from the 3×3 pixel area directly above the top-right corner of the field's outer rectangle. Its columns are `outer.right − 2` to `outer.right`, and its rows are `outer.top − 3` to `outer.top − 1`.

#### Scenario: Mark position for a field
- **WHEN** a field's outer rectangle is (156,22)–(193,50)
- **THEN** its mark area is columns 191 to 193 and rows 19 to 21

#### Scenario: No room above the field
- **WHEN** a field's outer top edge is at row 0, 1 or 2
- **THEN** the field has no mark area and its type is `image`

### Requirement: Default type is image
A field's type SHALL be `image` when its mark area has no pixels, holds one color only, holds a shape that no registered type owns, or uses the frame color `#BADBA1` as its mark color. An `image` field has no value.

#### Scenario: No mark
- **WHEN** the mark area is all white
- **THEN** the field's type is `image`

#### Scenario: Unrecognized shape
- **WHEN** the mark area holds green pixels in the shape `#.#` / `.#.` / `#.#`
- **THEN** the field's type is `image`

#### Scenario: Plus-shaped mark
- **WHEN** the mark area holds yellow pixels in the shape `.#.` / `###` / `.#.`
- **THEN** the field's type is `ref`, not `image`, because the plus shape is now a recognized shape

#### Scenario: Frame-colored mark
- **WHEN** the mark area holds the `float` shape in the frame color
- **THEN** the field's type is `image`

### Requirement: Reference image types
The fields of `testdata/example1.png` SHALL have these types and values, and SHALL keep them as the detection evolves.

#### Scenario: example1.png types and values
- **WHEN** `testdata/example1.png` is analyzed
- **THEN** the field with outer rectangle (101,22)–(137,47) is a `counter` with value 5
- **AND** the field at (156,22)–(193,50) is a `float` with 196 of its 532 data-area pixels in the mark color, a value of about 36.84%
- **AND** the field at (130,69)–(167,97) is a `counter` with value 3
- **AND** the fields at (35,23)–(77,52) and (72,70)–(107,97) are `image`

### Requirement: Second reference image types
The bool fields of `testdata/example2.png` SHALL have these values, and SHALL keep them as the detection evolves.

#### Scenario: example2.png bool fields
- **WHEN** `testdata/example2.png` (539×348) is analyzed
- **THEN** the field with outer rectangle (310,29)–(327,49) is a `bool` with value true
- **AND** the field at (288,31)–(305,49) is a `bool` with value false

### Requirement: Registered field types
The library SHALL keep a list of registered field types, currently `image`, `float`, `counter`, `bool`, `ref` and `string`. Each type SHALL have a name, its mark shape (except `image`, which has none), how its value is read, its value form, and how a value is written or that it cannot be written. Each type's behavior is defined in its own spec, `types/<name>`.

#### Scenario: Types of the library
- **WHEN** a program asks the library for its registered field types
- **THEN** it gets `image`, `float`, `counter`, `bool`, `ref` and `string`, each with its name and mark shape

### Requirement: Distinct mark shapes
No two registered types SHALL have the same mark shape, so a mark always selects at most one type. Every mark shape SHALL include the top-middle pixel, from which the mark color is read.

#### Scenario: Shapes are unique
- **WHEN** the mark shapes of all registered types are compared
- **THEN** every shape differs from every other one, and each includes the top-middle pixel

### Requirement: One module and one spec per type
Each field type SHALL be defined in its own spec, `types/<name>`, and implemented in its own module under the library's `types` folder, holding its mark shape, value reading, value form and writing. Adding a type SHALL need only a new spec, a new module, and one entry in the type list; no other module of the library, the CLI or the server SHALL name a specific type. (The web UI's per-type editors are outside this rule.)

#### Scenario: Type knowledge stays in its module
- **WHEN** the library, CLI and server sources are searched for code that handles a specific type (its mark, value, value form or writes)
- **THEN** it is found only in that type's module, the type list, and tests

### Requirement: Mark recognition
The mark color SHALL be the exact color of the top-middle pixel of the mark area. The mark's shape is the set of the 9 pixels that have exactly that color. A field's type SHALL be the registered type whose mark shape equals the field's mark shape. Each type's shape is given in its spec under `types/`.

#### Scenario: Shape selects the registered type
- **WHEN** a field's mark area holds a color in the shape that `types/counter` defines, and white pixels elsewhere
- **THEN** the field's type is `counter` and its mark color is that color

#### Scenario: Mark color from the top-middle pixel
- **WHEN** a field's mark area holds a red shape and its top-middle pixel is red
- **THEN** the mark color is red, and only the red pixels form the shape
