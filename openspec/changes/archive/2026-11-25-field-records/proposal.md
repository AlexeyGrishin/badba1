# Proposal

## Why

Fields hold single values, but a database needs rows: groups of values that belong together. In badba1, linking fields with lines already groups them visually, so a group of linked fields becomes a **record**. A record's fields need a stable order (a "column" order), and it has to be derivable from the drawing alone, so the order rules are based on connections and positions.

## What Changes

- A **record** is a set of fields connected through links, directly or through other fields: a connected component of the link graph. Every field belongs to exactly one record. A field with no links is a one-field record.
- A link that joins 3 or more fields connects each pair of them.
- Field order within a record is a depth-first walk:
  1. Start with the record's field with the smallest y, and the smallest x among equal y.
  2. From the current field, visit its connected fields that haven't been visited yet, one at a time, in order of smallest y, then smallest x. Each visited field continues with its own connections first (depth-first) before the walk moves on to the next field connected to the current one. A field with only one unvisited connection simply continues with that field.
  3. Each field appears exactly once, even when the links form loops.
- A field's position (x, y) is the top-left corner of its frame (outer rectangle), the same point that sets field numbers.
- Records are numbered R1, R2, … in order of their first field's position (y, then x).
- `cli` reports the record count in its summary line and lists each record with its ordered fields.
- Expected result for `testdata/example1.png`:
  - **R1 = F1, F2, F3**: dots box, then green box (same y as the dots box, larger x), then smiley.
  - **R2 = F4, F5**: red dots, then frown.

## Capabilities

### New Capabilities
- `records`: What a record is (fields connected by links), how field order inside a record is determined, and how records are numbered.

### Modified Capabilities
- `cli`: The summary line also reports the record count, and the output gains a record listing.

## Impact

- `crates/backend`: new `Record` type and `Layout.records`, computed in `detect` from fields and links. This is a breaking addition to `Layout` (only `cli` constructs one, in a unit test). No new dependencies.
- `crates/cli`: the summary line gains `, N records`, and there's a new `Records:` section. Existing tests that check the summary line are updated.
- Tests: synthetic unit tests for each ordering rule, plus reference-image expectations.
