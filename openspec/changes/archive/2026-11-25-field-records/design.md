# Design

## Context

`detect` returns `Layout { width, height, fields, links }`. Fields are numbered by `(outer.top, outer.left)`, and each link holds a sorted list of the field ids it joins. Records are derived purely from these two lists; no pixel access is needed. Requirements are in `specs/records/spec.md`, and the cli output in `specs/cli/spec.md`.

## Goals / Non-Goals

**Goals:**
- A deterministic record order that depends only on the drawing: link topology and field positions.
- Cheap computation: O(fields + links) after sorting.

**Non-Goals:**
- Record "schemas", meaning matching records against each other or naming their columns.
- Direction on links: links have no arrowheads, so connections are undirected.

## Decisions

**Field position order = field id order.** Fields are already numbered by `(outer.top, outer.left)`, which is exactly the spec's position order (y, then x, of the frame's top-left corner). So "earliest position" means "smallest field id". This lets the algorithm work on ids alone, and it stays correct as long as field numbering keeps that rule. The ordering helper is written in terms of ids, with a comment pointing at this equivalence, and a unit test pins it down.

**Algorithm (new module `records.rs`):**
1. Build an undirected adjacency list `Vec<BTreeSet<usize>>`, indexed by field id. For every link, add each pair of its fields in both directions, which also handles links joining 3+ fields. Using `BTreeSet` gives sorted neighbors with no duplicates, so parallel links between the same pair don't matter.
2. Keep a `visited` array. Go through field ids in ascending order; each unvisited id starts a new record. Its first field is the smallest id in its component, because every smaller id has already been visited by an earlier record.
3. Walk depth-first from the start, recursively in effect: append the field when it is first visited, then visit each neighbor in ascending order, skipping visited ones. It's implemented with an explicit stack of (field, neighbor iterator) frames, so a long chain of fields can't overflow the call stack. A neighbor that a deeper branch has already visited is skipped when its turn comes, which is what the "Loop of three fields" scenario requires.
4. Records are numbered in the order they are started, which is by first field id, and therefore by first field position.

*Alternative:* breadth-first search, or ordering all of a record's fields by position. Rejected because the spec asks for deep traversal: "continues with its connections".

**Data model:**
```rust
pub struct Record { pub id: usize, pub fields: Vec<usize> }   // fields in record order
pub struct Layout { ..., pub records: Vec<Record> }
```
`fn records(fields: &[Field], links: &[Link]) -> Vec<Record>` is public. It is pure and can be unit-tested with hand-built fields and links, without drawing images.

**`cli` output:** the summary line becomes `…: 539x348, 5 fields, 3 links, 2 records`. After `Links:` there is a new section:
```
Records:
  R1  F1, F2, F3
  R2  F4, F5
```

**Tests:**
- Unit tests in `records.rs` build `Field` values (only `id` and `outer` matter) and `Link` values directly, one test per spec scenario, plus a long-chain test (10,000 fields) to show there's no stack overflow.
- Reference expectations go in `tests/example1.rs`.
- `cli` tests update the summary line and check the records section, and a unit test of `render` checks a lone field.

## Risks / Trade-offs

- [The id-equals-position shortcut breaks if field numbering ever changes] → The unit test "position order equals id order" fails loudly if numbering and position order drift apart. The `records` function documents that assumption.
- [Very large graphs] → The walk uses explicit stacks, so it's iterative and linear.
- [The `Layout` struct gains a field, which breaks code that builds `Layout` directly] → Only the `cli` unit test does, and it's updated here.
