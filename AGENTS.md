# AGENTS.md — Coding Agent Guidelines for `mimimi`

This file provides instructions for agentic coding tools working in this repository.
The project is a Rust binary crate using **Rust edition 2024** and the stable toolchain (1.85+).

---

## Project Overview

- **Language:** Rust (edition 2024)
- **Type:** Binary crate (`src/main.rs`)
- **Build system:** Cargo
- **External dependencies:** None (add to `Cargo.toml` as needed)

---

## Commands

### Build

```bash
cargo build           # Debug build
cargo build --release # Optimized release build
cargo check           # Fast type-check without producing binaries
```

### Run

```bash
cargo run             # Build and run debug binary
cargo run --release   # Build and run release binary
```

### Test

```bash
cargo test                          # Run all tests
cargo test <name>                   # Run tests whose name contains <name>
cargo test <module>::<test_fn>      # Run a specific test by full path
cargo test -- --nocapture           # Show stdout/stderr from passing tests
cargo test --test <integration>     # Run a specific integration test binary
```

Examples:
```bash
cargo test parser                   # All tests with "parser" in the name
cargo test network::test_connect    # Specific test in a module
```

### Lint

```bash
cargo clippy                        # Run Clippy linter (default lints)
cargo clippy -- -D warnings         # Treat all warnings as errors (use in CI)
cargo clippy --all-targets          # Also lint tests and examples
```

### Format

```bash
cargo fmt                           # Format all source files in place
cargo fmt --check                   # Check formatting without modifying (use in CI)
```

### Full CI-equivalent check

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

---

## Code Style Guidelines

### Formatting

- All code **must** pass `cargo fmt` with default settings (no `rustfmt.toml` overrides).
- Do not manually format code; rely on `cargo fmt` entirely.
- Line length defaults to 100 characters (rustfmt default).
- Use trailing commas in multi-line expressions where `rustfmt` inserts them.

### Naming Conventions

| Item | Convention | Example |
|---|---|---|
| Functions and methods | `snake_case` | `parse_input()` |
| Variables and parameters | `snake_case` | `byte_count` |
| Modules and files | `snake_case` | `src/http_client.rs` |
| Structs, enums, traits | `PascalCase` | `HttpClient`, `ParseError` |
| Enum variants | `PascalCase` | `Error::NotFound` |
| Constants and statics | `SCREAMING_SNAKE_CASE` | `MAX_RETRIES` |
| Type aliases | `PascalCase` | `type Result<T> = std::result::Result<T, Error>;` |
| Crate name | `snake_case` | `mimimi` |

### Imports (`use` statements)

Group imports in the following order, with a blank line between groups:

1. Standard library (`std::`, `core::`, `alloc::`)
2. External crate imports (from `Cargo.toml` dependencies)
3. Local crate imports (`crate::`, `super::`, `self::`)

Example:
```rust
use std::collections::HashMap;
use std::io::{self, Read};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::config::Config;
use crate::error::Error;
```

- Prefer explicit imports over glob imports (`use foo::*`), except in `#[cfg(test)]` modules where `use super::*;` is acceptable.
- Merge related imports from the same module using brace syntax: `use std::io::{self, Read, Write};`.

### Types and Type Annotations

- Function signatures **must** have explicit types on all parameters and return values.
- Let local inference handle variable bindings unless the type is ambiguous or documentation value is high.
- Prefer `impl Trait` in function arguments for flexibility; use concrete types or generics when the caller needs to know the type.
- Use `type` aliases to avoid repeating complex types (e.g., `type Result<T> = std::result::Result<T, Error>;`).
- Avoid unnecessary `clone()` calls; prefer borrowing (`&T`, `&mut T`) where ownership is not required.

### Error Handling

- Use `Result<T, E>` for fallible operations. Do **not** use `unwrap()` or `expect()` in production code paths; reserve them for tests or cases where a panic is intentional and documented.
- Propagate errors with the `?` operator instead of manual `match` where possible.
- Define a crate-level error type (e.g., an `enum Error` or using a library like `thiserror`) as the project grows.
- Use `anyhow` for application-level error context and `thiserror` for library-level typed errors.
- Provide meaningful context in error messages; avoid bare "operation failed" strings.

Example:
```rust
fn read_config(path: &Path) -> Result<Config, Error> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| Error::Io { path: path.to_owned(), source: e })?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}
```

### Module and File Organization

- Follow the standard Cargo layout:
  - `src/main.rs` — binary entry point (keep thin; delegate to library code)
  - `src/lib.rs` — library root if the crate exposes a public API
  - `src/<module>.rs` or `src/<module>/mod.rs` — submodules
  - `tests/` — integration tests
  - `benches/` — benchmarks (using Criterion or similar)
  - `examples/` — runnable examples
- Keep `main.rs` and `lib.rs` entry points short; push logic into named modules.
- One primary concept per module; split large modules into submodules before they exceed ~400 lines.

### Documentation

- Document all public items (`pub fn`, `pub struct`, `pub enum`, etc.) with `///` doc comments.
- Use `//!` for module-level documentation at the top of the file.
- Include at least a one-line summary; add longer descriptions and `# Examples` sections for non-trivial APIs.
- Run `cargo doc --open` to preview rendered documentation.

### Testing

- Place unit tests in a `#[cfg(test)]` module at the bottom of each source file:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_something() {
          assert_eq!(1 + 1, 2);
      }
  }
  ```
- Place integration tests in `tests/` as separate files; they test the public API only.
- Use descriptive test names: `test_<what>_<expected_result>` (e.g., `test_empty_input_returns_error`).
- Use `#[should_panic(expected = "...")]` when testing expected panics.
- Prefer `assert_eq!` and `assert_ne!` over bare `assert!` where possible for better failure messages.

### Clippy

- All code must pass `cargo clippy` with no warnings.
- Do not suppress Clippy lints with `#[allow(...)]` without a comment explaining why.
- Acceptable suppression example:
  ```rust
  #[allow(clippy::too_many_arguments)] // This function mirrors a C API signature
  ```

### Unsafe Code

- Avoid `unsafe` blocks unless strictly necessary (e.g., FFI, performance-critical low-level code).
- Every `unsafe` block must have a `// SAFETY:` comment explaining the invariants being upheld.

### Panics

- Document functions that may panic with a `# Panics` section in their doc comment.
- Prefer returning `Result` or `Option` over panicking in library code.

---

## General Agent Instructions

- Always run `cargo fmt` before finishing edits to any `.rs` file.
- Always run `cargo clippy` after making code changes and fix all warnings before completing a task.
- Always run `cargo test` to verify nothing is broken after changes.
- When adding dependencies, use `cargo add <crate>` (requires `cargo-edit`) or edit `Cargo.toml` directly; always commit `Cargo.lock`.
- Do not modify `Cargo.lock` manually.
- Keep commits focused; one logical change per commit.
