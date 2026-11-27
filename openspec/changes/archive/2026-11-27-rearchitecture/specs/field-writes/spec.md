# Spec Delta

## MODIFIED Requirements

### Requirement: Wrong field or type
A write SHALL fail when no field has the given number, when the field's type cannot be written (as its spec under `types/` says; currently only `ref`), or when the value's type differs from the field's type.

#### Scenario: Unknown field
- **WHEN** a value is written to field number 99 in an image with 11 fields
- **THEN** the write fails with an error naming field 99

#### Scenario: Reference field
- **WHEN** any value is written to a `ref` field
- **THEN** the write fails with an error saying the field is not writable

#### Scenario: Type mismatch
- **WHEN** a text value is written to a `counter` field
- **THEN** the write fails with an error naming both types

## REMOVED Requirements

### Requirement: Counter writes
**Reason**: Moved to the `types/counter` capability so all `counter` behavior lives in one spec.
**Migration**: See `types/counter`, requirement "Counter writes" (unchanged text and scenarios).

### Requirement: Float writes
**Reason**: Moved to the `types/float` capability so all `float` behavior lives in one spec.
**Migration**: See `types/float`, requirement "Float writes" (unchanged text and scenarios).

### Requirement: Bool writes
**Reason**: Moved to the `types/bool` capability so all `bool` behavior lives in one spec.
**Migration**: See `types/bool`, requirement "Bool writes" (unchanged text and scenarios).

### Requirement: String writes
**Reason**: Moved to the `types/string` capability so all `string` behavior lives in one spec.
**Migration**: See `types/string`, requirement "String writes" (unchanged text and scenarios).

### Requirement: String write errors
**Reason**: Moved to the `types/string` capability so all `string` behavior lives in one spec.
**Migration**: See `types/string`, requirement "String write errors" (unchanged text and scenarios).

### Requirement: Image writes
**Reason**: Moved to the `types/image` capability so all `image` behavior lives in one spec.
**Migration**: See `types/image`, requirement "Image writes" (unchanged text and scenarios).
