# Spec Delta

## Purpose

Defines the bitmap font used to read text: an image of 3×5 glyph cells plus a list of characters. Together they are data the user edits outside the code, including alternate forms of a character.

## ADDED Requirements

### Requirement: Font files
The font SHALL consist of two files in `crates/backend/font/`: `font.png`, the glyph image, and `chars.txt`, the characters of the image's first glyph row in order. Both are built into the backend, so a changed font takes effect after a rebuild.

#### Scenario: Font is built in
- **WHEN** the backend is built from the repository
- **THEN** reading string fields uses the glyphs in `crates/backend/font/font.png` and the characters in `crates/backend/font/chars.txt`, without reading any file at run time

### Requirement: Glyph grid
`font.png` SHALL be read as a grid of glyph cells 3 pixels wide and 5 tall. Cell column c starts at x = 4·c and cell row r at y = 6·r, so one empty column and one empty row separate the cells. The color of the image's top-left pixel is the background. Every pixel of another color is part of a glyph.

#### Scenario: Cell positions
- **WHEN** the font image is 160×11 pixels
- **THEN** it has 40 cell columns and 2 cell rows, with the cell in column 2 of row 1 covering x 8–10 and y 6–10

### Requirement: Characters and alternate forms
The character at position c of `chars.txt` (counting characters, not bytes, and ignoring a trailing line break) SHALL be the character of cell column c. The glyph in row 0 is its main form. A non-empty cell in the same column of any lower row is an alternate form of the same character. Empty cells are ignored.

#### Scenario: Main and alternate E
- **WHEN** column 4 holds `###` / `#..` / `##.` / `#..` / `###` in row 0 and `###` / `#..` / `###` / `#..` / `###` in row 1, and character 4 of `chars.txt` is `E`
- **THEN** both shapes read as `E`

### Requirement: Valid font
The font SHALL have exactly one character in `chars.txt` per non-empty cell column of row 0. No glyph shape may belong to two different characters. A font that breaks either rule is rejected when it is loaded, with an error naming the problem.

#### Scenario: Shared shape
- **WHEN** the same glyph shape appears in the columns of two different characters
- **THEN** loading the font fails with an error naming both characters

#### Scenario: Count mismatch
- **WHEN** `chars.txt` lists 39 characters but row 0 has 40 non-empty cells
- **THEN** loading the font fails with an error stating both counts

### Requirement: Supplied font
The built-in font SHALL be the user-supplied image (160×11). Its characters are `ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890_+-*`, and it has alternate forms for `A E F G K R U 1 2 3 4 6 7 8 9 0`: 56 forms in total, all distinct.

#### Scenario: Supplied font loads
- **WHEN** the built-in font is loaded
- **THEN** it has 40 characters and 56 glyph forms, and no shape belongs to two characters
