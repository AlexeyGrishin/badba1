# Design

## Context

The repository currently contains only OpenSpec planning files. It is not a git repository yet. The installed toolchain is Cargo 1.99, which supports edition 2024 and resolver 3. See proposal.md - Why for motivation and specs/cargo-workspace/spec.md for the required behavior.

## Goals / Non-Goals

**Goals:**
- A workspace layout that new crates join by being created under `crates/`, with no edit to the root manifest.
- One place to set edition, version, license and shared dependency versions.

**Non-Goals:**
- No binary crate, CI pipeline, `rust-toolchain.toml`, rustfmt/clippy config or license file in this change.
- No real backend functionality. `backend` only gets a placeholder function and test.
- Not running `git init`. That stays the user's action.

## Decisions

**Virtual workspace (root `Cargo.toml` has `[workspace]` only, no `[package]`).**
Keeps the root free of source code and treats every crate as an equal member. Alternative: a root package that is also the workspace. Rejected because it ties the workspace to one crate and makes `cargo build` at the root build only that crate by default.

**`members = ["crates/*"]` glob.**
Satisfies "new crates join without editing the root manifest". Alternative: listing members explicitly. Rejected because it's one more step to forget for each crate. Trade-off: any directory under `crates/` with a `Cargo.toml` becomes a member, so scratch crates must live elsewhere.

**`resolver = "3"` and `edition = "2024"` in `[workspace.package]`.**
These are the current defaults for new projects on this toolchain. Resolver 3 is MSRV-aware when selecting dependency versions. Each member manifest uses `edition.workspace = true`, `version.workspace = true` and `license.workspace = true`.

**Empty `[workspace.dependencies]` table.**
Added now so later crates declare shared dependency versions once (`serde = { workspace = true }`). Costs nothing while empty.

**Initial version `0.1.0`, license `MIT OR Apache-2.0`, `publish = false`.**
These are placeholders the user can change. `publish = false` in `[workspace.package]` (inherited with `publish.workspace = true`) prevents an accidental `cargo publish` of internal crates.

**`backend` created as a library with a placeholder function and a unit test.**
`lib.rs` contains one small public function plus a `#[cfg(test)]` module, so `cargo test -p backend` has a real test to run. It is replaced when real backend work starts.

**Commit `Cargo.lock`.**
The workspace will contain binaries later, and Cargo's guidance is to commit the lockfile for reproducible builds. `.gitignore` therefore does not list it.

**`.gitignore` contents.**
`/target/`, rustfmt backups (`**/*.rs.bk`), debugger output (`*.pdb`), common editor folders (`.idea/`, `.vscode/`, `*.swp`) and OS files (`.DS_Store`, `Thumbs.db`). `.claude/settings.local.json` is also ignored because it holds machine-local settings. The rest of `.claude/` and all of `openspec/` stay tracked.

## Risks / Trade-offs

- [The glob picks up any stray `crates/*` folder that has a manifest] → Keep experiments outside `crates/`, or add `exclude` entries when needed.
- [Edition 2024 needs Rust ≥ 1.85] → The installed toolchain is newer. A `rust-toolchain.toml` can pin a version later if contributors are on older toolchains.
- [The placeholder license may not be what the user wants] → It's set in one place (`[workspace.package]`), so changing it is a single edit.
