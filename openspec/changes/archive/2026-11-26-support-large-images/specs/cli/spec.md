# Spec Delta

## MODIFIED Requirements

### Requirement: File errors
When the file cannot be read, is not a valid PNG, or is larger than the maximum image size, `cli` SHALL print an error naming the file to standard error and exit with status 1.

#### Scenario: Missing file
- **WHEN** a user runs `cli does-not-exist.png`
- **THEN** an error mentioning `does-not-exist.png` is printed to standard error and the exit status is 1

#### Scenario: Image too large
- **WHEN** a user runs `cli huge.png` on a valid 20001×20000 PNG
- **THEN** an error that mentions `huge.png`, says the image is too large and gives the 400,000,000-pixel maximum is printed to standard error, and the exit status is 1

#### Scenario: Maximum-size image
- **WHEN** a user runs `cli db/big.png` on the 10000×40000 image from the `gen_big` example
- **THEN** the layout is printed, starting `db/big.png: 10000x40000`, and the exit status is 0
