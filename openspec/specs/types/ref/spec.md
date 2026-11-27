# types/ref Specification

## Purpose

Defines the `ref` data type: its `+` type mark, how a reference to another record is traced along a line drawn in the mark's color, which record it targets or when it is null, how the value is shown, and that references cannot be written.

## Requirements

### Requirement: Ref mark
A field's type SHALL be `ref` when its mark shape is `.#.` / `###` / `.#.` (rows top to bottom, `#` = mark color).

#### Scenario: Ref mark
- **WHEN** the mark area holds yellow (255,201,14) pixels in the shape `.#.` / `###` / `.#.`, and white pixels elsewhere
- **THEN** the field's type is `ref` and its mark color is (255,201,14)

### Requirement: Ref data area is ignored
The data area of a `ref` field SHALL have no meaning: whatever is drawn in it does not affect the field's type or reference.

#### Scenario: Drawing inside a ref field
- **WHEN** a `ref` field with a yellow mark has yellow and black pixels in its data area
- **THEN** it is still a `ref` field, and its reference is the same as with an empty data area

### Requirement: Reference line pixels
A `ref` field's line SHALL consist of pixels that are exactly the field's mark color and lie outside the outer rectangle of every field. The mark's own pixels are where the line starts.

#### Scenario: Line in the mark color
- **WHEN** a `ref` field has a yellow mark and a yellow line runs from the mark to another field
- **THEN** the yellow pixels form the field's line

#### Scenario: Other colors are not part of the line
- **WHEN** a `ref` field has a yellow mark and a purple line runs from the mark to another field
- **THEN** the field's line consists of the mark only

#### Scenario: Pixels inside fields are not part of the line
- **WHEN** yellow pixels are drawn inside another field's data area near a yellow `ref` line
- **THEN** those pixels are not part of the line

### Requirement: Gaps of up to 4 pixels
Two line pixels SHALL be connected when they are at most 5 pixels apart horizontally and at most 5 pixels apart vertically, so a gap of up to 4 pixels of any other color does not break the line. The line is every line pixel connected to the mark, directly or through other line pixels.

#### Scenario: Gap of 4 pixels
- **WHEN** a horizontal reference line has a gap of 4 white pixels
- **THEN** both parts belong to the same line

#### Scenario: Gap of 5 pixels
- **WHEN** a horizontal reference line has a gap of 5 white pixels
- **THEN** the part beyond the gap does not belong to the line

#### Scenario: Crossed by another line
- **WHEN** a purple reference line is crossed by a yellow line painted over it, leaving a 1-pixel gap in the purple
- **THEN** the purple line continues past the crossing

### Requirement: Touching a field
A line SHALL touch a field when some line pixel is at most 5 pixels away horizontally and at most 5 pixels away vertically from a pixel of that field's frame. The line may stop up to 4 pixels short of a frame.

#### Scenario: Line ends at the frame
- **WHEN** the last pixel of a line is next to a frame pixel of field G
- **THEN** the line touches G

#### Scenario: Line stops 4 pixels short
- **WHEN** a line ends with 4 white pixels between its last pixel and the frame of field G
- **THEN** the line touches G

#### Scenario: Line stops 5 pixels short
- **WHEN** a line ends with 5 white pixels between its last pixel and the frame of field G
- **THEN** the line does not touch G

### Requirement: Reference target is a record
The target of a reference SHALL be a record, not a field. Records whose fields the line touches are candidates, except the referencing field's own record, which is always ignored. When exactly one candidate record remains, it is the target.

#### Scenario: Touching a field of another record
- **WHEN** a `ref` field in record 1 has a line touching field G, and G is in record 2
- **THEN** the reference's target is record 2

#### Scenario: Touching any field of the record
- **WHEN** a line touches the second or last field of record 2 instead of its first field
- **THEN** the reference's target is still record 2

#### Scenario: Touching several fields of one record
- **WHEN** a line touches two fields that are both in record 2
- **THEN** the reference's target is record 2

#### Scenario: Own record is ignored
- **WHEN** a line from a `ref` field in record 1 touches the field itself, another field of record 1, and a field of record 2
- **THEN** the reference's target is record 2

### Requirement: Null reference
A reference SHALL be null when no candidate record remains, or when two or more do.

#### Scenario: Line reaches nothing
- **WHEN** a `ref` field's line ends in empty space, or the field has no line beyond its mark
- **THEN** the reference is null

#### Scenario: Line reaches only its own record
- **WHEN** a line touches only fields of the referencing field's own record
- **THEN** the reference is null

#### Scenario: Line reaches two records
- **WHEN** a branching line touches a field of record 2 and a field of record 3
- **THEN** the reference is null

#### Scenario: Gap too wide before the target
- **WHEN** a line toward record 2 has a gap of 5 white pixels, so the part touching record 2 is not connected to the mark
- **THEN** the reference is null

### Requirement: Reference image references
The references in `testdata/example2.png` SHALL resolve as follows, and SHALL keep doing so as the detection evolves.

#### Scenario: example2.png references
- **WHEN** `testdata/example2.png` is analyzed
- **THEN** the field at (198,26)–(220,43) is a `ref` whose target is the record containing the field at (250,23)–(280,52)
- **AND** the field at (213,74)–(234,93) is a `ref` with the same target

### Requirement: Ref value form
A `ref` value's kind SHALL be a record reference: the target record's number, or null. The database handle gives the target record's id. Its CLI text SHALL be `ref R<number>` for a target, or `ref null`.

#### Scenario: Reference with a target
- **WHEN** a `ref` field's target is record 2, whose id is `250/23`
- **THEN** its value is a reference to record 2 with id `250/23`, and its CLI text is `ref R2`

#### Scenario: Null reference text
- **WHEN** a `ref` field's reference is null
- **THEN** its value is a null reference, and its CLI text is `ref null`

### Requirement: References are not writable
A write to a `ref` field SHALL fail with an error saying the field is not writable, whatever the value, and leave the image unchanged.

#### Scenario: Reference field
- **WHEN** any value is written to a `ref` field
- **THEN** the write fails with an error saying the field is not writable
