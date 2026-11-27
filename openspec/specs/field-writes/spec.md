# field-writes Specification

## Purpose

Defines how a value is written into a field of a badba1 image: which values each type accepts, how they are drawn in the data area, what the write returns, and when it fails. Field geometry is never changed.

## Requirements

### Requirement: Write a field's value
The backend SHALL write a value into one field, identified by its number in the image's detected layout. It changes pixels only inside that field's data area, and returns the field's value as read back from the modified image.

#### Scenario: Write and read back
- **WHEN** the value 3 is written to a `counter` field
- **THEN** the write returns `counter 3`, and analyzing the modified image gives the same fields, links and records as before, with that field's value now 3

#### Scenario: Pixels outside the data area are untouched
- **WHEN** any write succeeds
- **THEN** every pixel outside the written field's data area is unchanged

### Requirement: Clear and draw colors
A write SHALL first set every data-area pixel to the background color, which is the color of the image's top-left pixel. It then draws the new value in the field's mark color, or, for an `image` field, places the bitmap with its own colors.

#### Scenario: Old content is removed
- **WHEN** a `counter` field holding 5 dots is written with 2
- **THEN** the data area holds only the 2 new squares, in the mark color, on the background color

### Requirement: Failed writes change nothing
A write that fails SHALL return an error and leave the image unchanged.

#### Scenario: Error leaves image as it was
- **WHEN** a write fails for any reason, such as an overflow, an unknown character or a wrong type
- **THEN** the image's pixels are identical to before the write

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
