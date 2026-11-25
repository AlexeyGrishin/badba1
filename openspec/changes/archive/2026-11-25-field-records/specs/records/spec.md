# Spec Delta

## Purpose

Defines records: groups of fields connected through links. It also defines the order of fields within a record, based on connections and positions, and how records are numbered.

## ADDED Requirements

### Requirement: Records are connected fields
The system SHALL group fields into records. Two fields are in the same record when a chain of links connects them. A link that joins 3 or more fields connects each pair of them. Every field SHALL belong to exactly one record, and a field without links forms a record by itself.

#### Scenario: Linked fields form one record
- **WHEN** field A is linked to B, and B is linked to C
- **THEN** A, B and C form one record

#### Scenario: Unconnected groups form separate records
- **WHEN** A is linked to B, C is linked to D, and no link joins the two pairs
- **THEN** there are two records, {A, B} and {C, D}

#### Scenario: Lone field
- **WHEN** a field has no links
- **THEN** it forms a record with just that field

#### Scenario: Link joining three fields
- **WHEN** one link joins fields A, B and C
- **THEN** A, B and C form one record, with each connected to the other two

### Requirement: Field position
A field's position for record ordering SHALL be the top-left corner of its frame (outer rectangle). One position comes before another when its y is smaller, or when the y values are equal and its x is smaller.

#### Scenario: Smaller y comes first
- **WHEN** field A's frame starts at (156,22) and field B's at (35,23)
- **THEN** A comes before B

#### Scenario: Equal y, smaller x comes first
- **WHEN** field A's frame starts at (101,22) and field B's at (156,22)
- **THEN** A comes before B

### Requirement: First field of a record
A record's first field SHALL be its field with the earliest position.

#### Scenario: Topmost field starts the record
- **WHEN** a record contains fields at (35,23), (101,22) and (156,22)
- **THEN** its first field is the one at (101,22)

### Requirement: Depth-first field order
After the first field, the fields of a record SHALL follow a depth-first walk. From each field, the walk visits its connected fields that haven't been visited yet, one at a time and earliest position first. Each one's own connections are walked completely before moving on to the next field connected to the current one.

#### Scenario: Single connection continues the chain
- **WHEN** a record is the chain A–B–C, and A has the earliest position
- **THEN** the order is A, B, C, regardless of where B and C are

#### Scenario: Deeper fields come before later siblings
- **WHEN** A at (0,0) is linked to B at (100,40) and to C at (0,80), and B is also linked to D at (100,120)
- **THEN** the order is A, B, D, C: B comes before C because it is higher, and D, connected through B, comes before C even though D is lower than C

#### Scenario: Siblings with equal y
- **WHEN** A at (50,0) is linked to B at (100,40) and to C at (0,40)
- **THEN** the order is A, C, B

### Requirement: Each field appears once
Each field of a record SHALL appear exactly once in its order, even when links form loops. A field that is already in the order is skipped when it is reached again.

#### Scenario: Loop of three fields
- **WHEN** A at (0,0), B at (100,0) and C at (50,80) are linked in a loop A–B, B–C, C–A
- **THEN** the order is A, B, C, with each field once: C is reached from B before the walk returns to A

### Requirement: Record numbering
Records SHALL be numbered from 1 in order of their first field's position.

#### Scenario: Two records
- **WHEN** one record's first field is at (130,69) and another's is at (101,22)
- **THEN** the record starting at (101,22) is record 1, and the other is record 2

### Requirement: Reference image records
The records of `testdata/example1.png` SHALL be as follows, and SHALL stay so as the detection evolves.

#### Scenario: example1.png records
- **WHEN** `testdata/example1.png` is analyzed
- **THEN** record 1 has, in order, the fields at (101,22), (156,22) and (35,23)
- **AND** record 2 has, in order, the fields at (130,69) and (72,70)
