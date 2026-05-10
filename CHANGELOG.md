# Changelog

All notable changes to Nova are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Initial workspace setup (Phase 0)
- `nova-diagnostics`: error reporting infrastructure with codespan-reporting
- `nova-lexer`: full token set for Nova source files (logos-based)
- `nova-ast`: AST node type definitions
- `nova-parser`: recursive descent + Pratt parser (chumsky-based)
- `nova-cli`: `nova parse <file>` command for parser testing
- CI workflow (cargo test, fmt, clippy)
- Snapshot testing harness via `insta`
- Example files: hello-world, fibonacci
