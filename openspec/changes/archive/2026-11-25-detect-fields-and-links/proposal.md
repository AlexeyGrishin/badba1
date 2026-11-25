# Proposal

## Why

badba1 stores its data as a picture, so the first thing the system must do is understand the picture's structure: where the data boxes ("fields") are and which of them are connected. Everything later (reading data, editing it through an API) builds on this, so it comes first. A small command-line tool is needed alongside it so the detection can be checked by eye against real images drawn in an editor such as MS Paint.

## What Changes

- The `backend` library gains the ability to read a PNG file and detect, in color `#BADBA1`:
  - **Fields:** axis-aligned rectangles of any size, with any border thickness (each side may differ), drawn around a rectangular interior. The interior is the field's data area.
  - **Links:** gap-free lines of any width and shape, in the same color, that join two or more fields.
- The frame color must match `#BADBA1` exactly, fully opaque, with no tolerance. Keeping images free of near-miss pixels is the image author's job. Editors can introduce them where strokes overlap: the reference image had 4 `#BADAA1` pixels, which have since been fixed.
- A new runnable crate `cli` (at `crates/cli`) takes a PNG path as its argument and prints the detected fields (position, size, border thickness, data area) and the links between them.
- A copy of the user's `db/example1.png` becomes the reference test image at `testdata/example1.png`. It must yield exactly 5 fields and 3 links. `db/` stays the user's folder for manual testing and is ignored by git.
- Not included: reading or decoding what is drawn inside a field's data area, writing images, REST, and formats other than PNG.

## Capabilities

### New Capabilities
- `image-layout`: Detecting the structure of a badba1 database image: which fields exist, where their frames and data areas are, and which fields are linked.
- `cli`: The `cli` command-line tool and the commands it offers for inspecting a database image.

### Modified Capabilities
<!-- None. The new `cli` crate lives under crates/ and joins the workspace as the existing cargo-workspace spec already requires. -->

## Impact

- `crates/backend`: new public API for loading a PNG and returning the detected layout. Adds a dependency on the `image` crate (PNG decoding only).
- New crate `crates/cli` (binary `cli`), depending on `backend`.
- Root `Cargo.toml`: `image` and `backend` added to `[workspace.dependencies]`.
- `testdata/example1.png` gets committed and is used by the tests. `.gitignore` gains `/db/`.
- Needs network access once, so Cargo can download `image` and its dependencies.
