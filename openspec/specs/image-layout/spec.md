# image-layout Specification

## Purpose

Detects the structure of a badba1 database image: the fields (data boxes framed in the badba1 color), their frames and data areas, and the links drawn between them.

## Requirements

### Requirement: Frame color matching
A pixel SHALL count as frame color only when its color is exactly `#BADBA1` (red 186, green 219, blue 161) and it is fully opaque (alpha 255). There is no tolerance: every other pixel, however close in color, is background or data.

#### Scenario: Exact color matches
- **WHEN** a pixel is (186, 219, 161) with alpha 255
- **THEN** it counts as frame color

#### Scenario: Near-miss color does not match
- **WHEN** a pixel is (186, 218, 161) with alpha 255
- **THEN** it does not count as frame color

#### Scenario: Different color does not match
- **WHEN** a pixel is (34, 177, 76) with alpha 255
- **THEN** it does not count as frame color

#### Scenario: Partly transparent pixel does not match
- **WHEN** a pixel has RGB (186, 219, 161) and alpha 254
- **THEN** it does not count as frame color

### Requirement: Field detection
The system SHALL detect as a field every rectangular region of non-frame pixels that is completely surrounded by a frame-color frame. The region must not touch the image's edge. The frame's outer edge must also be a rectangle, and the frame must be at least 1 pixel thick on each side. Field size and the thickness of each side may be any value and may differ from side to side.

#### Scenario: Square frame with equal borders
- **WHEN** the image contains a 40×30 frame-color rectangle with a 5-pixel border on every side around a white interior
- **THEN** exactly one field is detected, with a 30×20 data area and border thicknesses of 5 on each side

#### Scenario: Unequal borders
- **WHEN** a frame has border thicknesses top 1, right 4, bottom 7 and left 2
- **THEN** one field is detected and reports those four thicknesses

#### Scenario: Data area holds a drawing
- **WHEN** the interior of a frame contains pixels of other colors, such as a drawing in black and red
- **THEN** the field is still detected, and its data area is the whole interior

#### Scenario: Thin-line field
- **WHEN** a frame is 1 pixel thick on every side around a 1×1 interior
- **THEN** one field is detected with a 1×1 data area

### Requirement: Non-rectangular enclosures are not fields
The system SHALL NOT report a field for a region enclosed by frame color unless that region is a filled axis-aligned rectangle. A region that contains frame-color pixels inside it is not filled.

#### Scenario: Closed curve
- **WHEN** a frame-color line forms a closed circle around an area of white pixels
- **THEN** no field is reported for the enclosed area

#### Scenario: Frame-color pixels in the interior
- **WHEN** a rectangular frame's interior contains a dot of frame color that does not touch the frame
- **THEN** no field is reported for that frame

### Requirement: Field identity and order
Each field SHALL be reported with a number, its outer rectangle, its data-area rectangle and its four border thicknesses. Fields are numbered from 1 in reading order: by the top edge of the outer rectangle, then by the left edge.

#### Scenario: Numbering follows reading order
- **WHEN** an image has field A with outer top-left (100, 22) and field B with outer top-left (35, 23)
- **THEN** A is field 1 and B is field 2

### Requirement: Link detection
Frame-color pixels that do not belong to any field's frame form strokes, where a stroke is a group of such pixels connected horizontally, vertically or diagonally. The system SHALL report a link for every stroke that touches the frames of two or more different fields, listing those fields in ascending order. A stroke of any width, shape or curvature qualifies. Links are numbered from 1 in order of their field lists.

#### Scenario: Straight thin link
- **WHEN** a 1-pixel-wide straight line joins the frame of field 1 to the frame of field 2
- **THEN** one link is reported between fields 1 and 2

#### Scenario: Thick curved link
- **WHEN** a 10-pixel-wide spiral stroke starts at the frame of field 4 and ends at the frame of field 5
- **THEN** one link is reported between fields 4 and 5

#### Scenario: Branching link
- **WHEN** one connected stroke touches the frames of fields 1, 2 and 3
- **THEN** one link is reported, listing fields 1, 2 and 3

#### Scenario: Stroke touching a single field
- **WHEN** a stroke touches the frame of only one field, or of none
- **THEN** no link is reported for it

### Requirement: Links need gap-free lines
A line with a gap SHALL NOT connect the fields at its two ends.

#### Scenario: Line with a gap
- **WHEN** a line from field 1 toward field 2 has a 1-pixel gap of white pixels across its whole width, so that the two pieces touch neither horizontally, vertically nor diagonally
- **THEN** no link is reported between fields 1 and 2

### Requirement: Reference image
The detected layout of the reference image `testdata/example1.png` SHALL be 5 fields and 3 links, and SHALL stay so as the detection evolves.

#### Scenario: example1.png layout
- **WHEN** `testdata/example1.png` (539×348) is analyzed
- **THEN** 5 fields are reported, each with a 5-pixel border on every side, with outer rectangles (left, top, right, bottom) (101,22,137,47), (35,23,77,52), (156,22,193,50), (130,69,167,97) and (72,70,107,97)
- **AND** 3 links are reported: between the fields at (35,23,77,52) and (101,22,137,47), between (101,22,137,47) and (156,22,193,50), and between (72,70,107,97) and (130,69,167,97)
