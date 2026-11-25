# Tasks

## 1. Workspace root

- [x] 1.1 Create root `Cargo.toml` as a virtual workspace: `members = ["crates/*"]`, `resolver = "3"`, `[workspace.package]` with `edition = "2024"`, `version = "0.1.0"`, `license = "MIT OR Apache-2.0"`, `publish = false`, and an empty `[workspace.dependencies]`. Verify the file has no `[package]` section.
- [x] 1.2 Create root `.gitignore` with `/target/`, `**/*.rs.bk`, `*.pdb`, `.idea/`, `.vscode/`, `*.swp`, `.DS_Store`, `Thumbs.db` and `.claude/settings.local.json`. Verify it does not list `Cargo.lock`.

## 2. backend library crate

- [x] 2.1 Create `crates/backend/Cargo.toml` with `name = "backend"` and `edition`, `version`, `license` and `publish` inherited via `.workspace = true`. Verify `cargo metadata --no-deps --format-version 1` at the root lists `backend` as a workspace member.
- [x] 2.2 Create `crates/backend/src/lib.rs` with a crate-level doc comment, one small public placeholder function and a `#[cfg(test)]` unit test for it. Verify `cargo test -p backend` runs at least one test and passes.

## 3. Workspace checks

- [x] 3.1 Run `cargo build --workspace` and `cargo test --workspace` from the repository root. Verify both succeed and `Cargo.lock` is generated at the root.
- [x] 3.2 Run `cargo fmt --all --check` and `cargo clippy --workspace -- -D warnings`. Verify both pass with no warnings.

## Workflow follow-up

- Run `git init` (user's choice) so `.gitignore` takes effect. Then `git status` should not show `target/`, and `git check-ignore Cargo.lock` should report nothing.
- Archive the change with `/openspec-archive-change` after review.
