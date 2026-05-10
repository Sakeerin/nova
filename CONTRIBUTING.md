# Contributing to Nova

Thank you for your interest in contributing!

## Development Setup

1. Install Rust (MSRV 1.78): https://rustup.rs
2. Clone the repo
3. `cargo build --workspace`
4. `cargo test --workspace`

## Commit Style

Use conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`

## Code Style

- `cargo fmt` and `cargo clippy --all-targets --all-features -- -D warnings` must pass
- No `unwrap()` outside tests
- All public items have rustdoc

## Pull Requests

- One logical change per PR
- Include tests for new functionality
- Update CHANGELOG.md
