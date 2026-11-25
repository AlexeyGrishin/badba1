# Proposal

## Why

The project is starting from an empty folder and will be written in Rust as several crates. A Cargo workspace is needed now, before any feature code is written, so every crate shares one build, one lockfile and one output directory, and new crates can be added without restructuring.

## What Changes

- Add a root `Cargo.toml` that defines a virtual Cargo workspace (no root package) and picks up member crates from `crates/*`.
- Add shared workspace settings (edition, resolver, package metadata) that member crates inherit.
- Add the first member crate, `backend`, as a library at `crates/backend`, containing a minimal public API and a unit test so the workspace builds and tests cleanly.
- Add a Rust `.gitignore` at the repository root that excludes build output and editor/OS clutter, and keeps `Cargo.lock` under version control.

## Capabilities

### New Capabilities
- `cargo-workspace`: How the repository's Rust crates are organized, built and tested as one Cargo workspace, including where new crates go and what is excluded from version control.

### Modified Capabilities
<!-- None: no specs exist yet. -->

## Impact

- New files: `Cargo.toml`, `.gitignore`, `crates/backend/Cargo.toml`, `crates/backend/src/lib.rs`.
- Tooling: requires a Rust toolchain that supports edition 2024 (Rust 1.85 or newer). The installed toolchain is Cargo 1.99.
- No external crate dependencies are added.
- The folder is not a git repository yet. The `.gitignore` takes effect once `git init` is run.
