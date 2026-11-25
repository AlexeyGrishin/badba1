# cargo-workspace Specification

## Purpose

Defines how the repository's Rust crates are organized, built and tested together as one Cargo workspace, where new crates are placed, and which generated files stay out of version control.

## Requirements

### Requirement: Single workspace build from the repository root
The repository SHALL be a Cargo workspace whose root manifest declares no package of its own, so that building or testing from the repository root covers every member crate.

#### Scenario: Build all crates from the root
- **WHEN** a developer runs `cargo build --workspace` in the repository root
- **THEN** every member crate compiles and the command exits successfully

#### Scenario: Test all crates from the root
- **WHEN** a developer runs `cargo test --workspace` in the repository root
- **THEN** the tests of every member crate run and the command exits successfully

### Requirement: Member crates live under crates/
Every member crate SHALL live in its own directory directly under `crates/`, and the workspace SHALL include such a directory as a member without the root manifest listing it by name.

#### Scenario: Adding a new crate
- **WHEN** a developer creates a new crate at `crates/<name>` with its own `Cargo.toml`
- **THEN** `cargo metadata` in the repository root lists that crate as a workspace member without any edit to the root `Cargo.toml`

### Requirement: Shared workspace settings
Member crates SHALL take their Rust edition, version and license from the workspace's shared package settings, so these are set in one place.

#### Scenario: Edition is inherited
- **WHEN** a member crate's manifest sets `edition.workspace = true`
- **THEN** the crate is compiled with the edition declared in the root manifest (2024)

### Requirement: backend library crate
The workspace SHALL contain a library crate named `backend` at `crates/backend` that other crates in the workspace can depend on.

#### Scenario: backend builds as a library
- **WHEN** a developer runs `cargo build -p backend`
- **THEN** a library target named `backend` is built and the command exits successfully

#### Scenario: backend has a passing test
- **WHEN** a developer runs `cargo test -p backend`
- **THEN** at least one test runs and all tests pass

### Requirement: Version control excludes build output
The repository SHALL have a root `.gitignore` that excludes Cargo build output and common editor and OS files, and SHALL keep `Cargo.lock` tracked.

#### Scenario: Build output is ignored
- **GIVEN** the repository is a git repository and `cargo build` has been run
- **WHEN** a developer runs `git status`
- **THEN** the `target/` directory is not reported as untracked

#### Scenario: Lockfile is tracked
- **GIVEN** the repository is a git repository and `Cargo.lock` exists
- **WHEN** a developer runs `git check-ignore Cargo.lock`
- **THEN** the file is reported as not ignored
