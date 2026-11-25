# Spec Delta

## Purpose

Defines how the text of a `string` field is read from its data area: which pixels are ink, how letters and lines are found, how letters are matched against the glyph font, and how spaces and unknown letters appear in the value.

## ADDED Requirements

### Requirement: Background and ink
The background color SHALL be the color of the whole image's top-left pixel. Inside a `string` field's data area, every pixel of another color is ink, whatever its color. Letters may use different colors.

#### Scenario: Letters in several colors
- **WHEN** a `string` field's data area holds blue letters `OPEN` followed by a pink `S` and blue `PEC`
- **THEN** its text is `OPENSPEC`

#### Scenario: Non-white background
- **WHEN** the image's top-left pixel is light gray and a string field's data area is light gray with black letters
- **THEN** the light gray pixels are background and the black pixels are ink

### Requirement: Letter cells
Each letter SHALL occupy a cell 3 pixels wide and 5 tall. In a line, the leftmost ink column not yet used starts a cell covering that column and the next two. The next letter starts at the first ink column after the cell.

#### Scenario: One-column gaps
- **WHEN** letters are drawn with one empty column between them
- **THEN** each letter is read from its own 3-column cell

#### Scenario: Touching letters
- **WHEN** two letters are drawn with no empty column between them
- **THEN** they are still read as two letters, from columns x..x+2 and x+3..x+5

### Requirement: Lines
A line's cells SHALL all start on the same row. That row is set by letters whose ink is exactly 5 rows tall: such a letter's top row starts a line. Shorter marks such as `-` belong to the line whose 5 rows contain them. Letters starting on a different row form a different line, and a field may hold several lines.

#### Scenario: Two lines
- **WHEN** a string field holds `AB` with its top row at y=10 and `CD` with its top row at y=17
- **THEN** its text is `AB` and `CD` on separate lines

#### Scenario: Short glyph in a line
- **WHEN** a line holds `A-B`, where the `-` is drawn only on the line's third row
- **THEN** the text is `A-B`

### Requirement: Line order and joining
Lines SHALL be ordered by their top row, then by their leftmost ink column, and joined with a line break (`\n`) into the field's text. A field with no ink has the empty text.

#### Scenario: Multi-line text
- **WHEN** a field holds `TOP` on a higher row and `BOTTOM` on a lower row
- **THEN** its text is `TOP\nBOTTOM`

#### Scenario: Lines beside each other
- **WHEN** a field holds `LEFT` starting at row 5 near its left edge, and `RIGHT` starting at row 12 near its right edge
- **THEN** its text is `LEFT\nRIGHT`, because `LEFT` starts higher

#### Scenario: Empty field
- **WHEN** a string field's data area has only background pixels
- **THEN** its text is empty

### Requirement: Glyph matching
A letter's 3×5 cell SHALL read as the character whose font glyph, main or alternate form, matches the cell's ink exactly. A cell that matches no form reads as `�` (U+FFFD). Pixels of other lines are not part of the cell.

#### Scenario: Both forms of E
- **WHEN** one E is drawn with middle row `##.` and another with middle row `###`
- **THEN** both read as `E`

#### Scenario: Unknown glyph
- **WHEN** a cell's shape matches no form in the font
- **THEN** it reads as `�`

#### Scenario: One pixel off
- **WHEN** a cell matches the main `A` except for one extra pixel
- **THEN** it reads as `�`, because only exact matches count

### Requirement: Spaces
Between two consecutive letters of a line, a gap of G empty columns SHALL be read as floor(G / 3) spaces. Gaps before the first letter and after the last letter of a line add no spaces.

#### Scenario: Small gaps
- **WHEN** two letters are separated by 1 or 2 empty columns
- **THEN** no space is read between them

#### Scenario: One space
- **WHEN** `A` and `B` are separated by 3, 4 or 5 empty columns
- **THEN** the text is `A B`

#### Scenario: Two spaces
- **WHEN** `A` and `B` are separated by 6 empty columns
- **THEN** the text is `A  B`

#### Scenario: Leading and trailing gaps
- **WHEN** a line's first letter starts 10 columns from the data area's left edge, and its last letter ends 10 columns before the right edge
- **THEN** the text has no leading or trailing spaces

### Requirement: Reference image string
The string field of `testdata/example3.png` SHALL read as follows, and SHALL keep doing so as detection evolves.

#### Scenario: example3.png string field
- **WHEN** `testdata/example3.png` is analyzed
- **THEN** the field at (297,54)–(338,60) is a `string` with text `OPENSPEC`
