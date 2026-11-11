# Contributing to Nova

Thank you for your interest in contributing!

## Development Setup

1. Install Rust (MSRV 1.78): https://rustup.rs
2. Clone the repo
3. `cargo build --workspace`
4. `cargo test --workspace`
5. `cargo test --workspace --test selfhost`: the self-hosting
   differential, which `cargo test --workspace` skips (its target says
   `test = false`) and CI runs in a job of its own
   (docs/adr/0055-the-self-hosted-item-collection-and-the-hir-dump.md §15)

## Commit Style

Use conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`

## Code Style

- `cargo fmt` and `cargo clippy --all-targets --all-features -- -D warnings` must pass, and
  `cargo clippy --all-features --test selfhost -- -D warnings`, since
  `--all-targets` skips that target
- No `unwrap()` outside tests
- All public items have rustdoc

## Pull Requests

- One logical change per PR
- Include tests for new functionality
- Update CHANGELOG.md
