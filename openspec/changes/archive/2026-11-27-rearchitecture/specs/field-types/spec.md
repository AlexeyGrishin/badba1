# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: Mark color and shape
**Reason**: Replaced by "Mark recognition", which matches the mark against the registered types; each type's shape and its scenarios moved to that type's spec.
**Migration**: See "Mark recognition" in this capability, and "Float mark", "Counter mark", "Bool mark", "Ref mark" and "String mark" in `types/float`, `types/counter`, `types/bool`, `types/ref` and `types/string` (unchanged shapes and scenarios).

### Requirement: Float value
**Reason**: Moved to the `types/float` capability so all `float` behavior lives in one spec.
**Migration**: See `types/float`, requirement "Float value" (unchanged text and scenarios).

### Requirement: Counter value
**Reason**: Moved to the `types/counter` capability so all `counter` behavior lives in one spec.
**Migration**: See `types/counter`, requirement "Counter value" (unchanged text and scenarios).

### Requirement: Bool value
**Reason**: Moved to the `types/bool` capability so all `bool` behavior lives in one spec.
**Migration**: See `types/bool`, requirement "Bool value" (unchanged text and scenarios).

### Requirement: Ref data area is ignored
**Reason**: Moved to the `types/ref` capability so all `ref` behavior lives in one spec.
**Migration**: See `types/ref`, requirement "Ref data area is ignored" (unchanged text and scenarios).
