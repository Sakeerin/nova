# 50 — Testing & CI Strategy

> Phase: All (testing infra grows with each phase)

---

## 1. Test Categories

### 1.1 Compile-pass tests
Location: `tests/compile-pass/`
- Each `.nova` file must successfully type-check + codegen
- One file per case
- Categorized by feature: `tests/compile-pass/generics/`, `tests/compile-pass/async/`, etc.

### 1.2 Compile-fail tests
Location: `tests/compile-fail/`
- Each file must fail to compile
- Adjacent `.stderr` snapshot file contains expected error output (exact match modulo paths)
- Use `insta` for managing snapshots
- Verify error code, span, and message format

Example layout:
```
tests/compile-fail/
├── E0010-type-mismatch/
│   ├── basic.nova
│   ├── basic.stderr
│   ├── nested.nova
│   └── nested.stderr
└── E0020-non-exhaustive/
    ├── enum.nova
    └── enum.stderr
```

### 1.3 Runtime tests
Location: `tests/runtime/`
- Each file is compiled + executed
- Adjacent `.stdout` (or `.stdout.json`) is expected output
- Covers behavior, not just types

### 1.4 Stdlib tests
Location: per-module: `std/<module>/tests/`
- Written in Nova
- Run via `nova test`

### 1.5 UI tests (frontend)
Location: `tests/ui/`
- WASM tests via `wasm-bindgen-test`
- Headless browser tests via Playwright

### 1.6 Property tests
Throughout, in `tests/proptest/`:
- Lexer: roundtrip
- Parser: random AST → format → parse → equality
- Type system: well-typed program never panics during typecheck

### 1.7 Fuzz tests
Location: `fuzz/fuzz_targets/`
- `lex.rs`: feed random bytes to lexer
- `parse.rs`: feed random tokens to parser
- `typeck.rs`: feed random AST to type checker
- `json.rs`: feed random bytes to JSON parser
- Run via `cargo fuzz run <target>` continuously in CI

### 1.8 Benchmarks
Location: `benches/` per crate
- Use `criterion`
- Track over time via `bencher.dev` or similar
- CI fails on regression > 5%

---

## 2. Test Harness

### 2.1 Compile-pass/fail/runtime harness
A single Rust integration test in `tests/harness.rs`:
```rust
#[test]
fn test_compile_pass() {
    for entry in walkdir::WalkDir::new("tests/compile-pass") {
        if entry.path().extension() == Some("nova") {
            run_compile_pass(entry.path());
        }
    }
}

#[test]
fn test_compile_fail() {
    for entry in walkdir::WalkDir::new("tests/compile-fail") {
        if entry.path().extension() == Some("nova") {
            run_compile_fail(entry.path());
        }
    }
}
```

### 2.2 Snapshot management
- Run `cargo insta review` to approve new snapshots
- CI fails on unaccepted snapshot changes
- Path normalization: replace absolute paths with `$DIR`

---

## 3. Test Coverage Requirements

| Component | Min coverage |
|---|---|
| nova-lexer | 95% |
| nova-parser | 90% |
| nova-typeck | 85% |
| nova-codegen | 75% (codegen tested via runtime tests) |
| stdlib | 80% |
| nova-runtime | 80% |

Use `cargo-tarpaulin` or `cargo-llvm-cov`.

---

## 4. CI Pipeline

### 4.1 On every PR (`.github/workflows/ci.yml`)
```yaml
jobs:
  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo fmt --check
      - run: cargo clippy --all-targets --all-features -- -D warnings

  test-linux:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [stable, beta]
    steps:
      - uses: actions/checkout@v4
      - run: rustup toolchain install ${{ matrix.rust }}
      - run: cargo test --workspace --all-features
      - run: cargo test --workspace --no-default-features

  test-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --workspace

  test-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --workspace

  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release -p nova-cli
      - run: ./target/release/nova test
        working-directory: examples/01-hello-world
      # ... for each example

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-llvm-cov
      - run: cargo llvm-cov --workspace --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v4

  docs:
    runs-on: ubuntu-latest
    steps:
      - run: cargo doc --workspace --no-deps
      # Build mdBook docs
      - run: cd docs/book && mdbook build

  bench:
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    steps:
      - run: cargo bench --workspace
      - run: ./scripts/check-perf-regression.sh
```

### 4.2 Nightly fuzz
```yaml
name: Fuzz
on:
  schedule:
    - cron: '0 0 * * *'
jobs:
  fuzz:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target: [lex, parse, typeck, json]
    steps:
      - run: cargo install cargo-fuzz
      - run: cargo fuzz run ${{ matrix.target }} -- -max_total_time=600
```

### 4.3 Release pipeline
On tag `v*`:
- Build binaries for: linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, windows-x86_64
- Upload to GitHub Releases
- Update install.sh / install.ps1 metadata
- Publish docs to `novalang.dev`

---

## 5. Performance Regression Tracking

- After each merge to main, run benches
- Store results in `gh-pages` branch as JSON
- Render trend graphs at `novalang.dev/perf`
- Alert in CI if any bench slows by > 5%

---

## 6. Bug Bounty / Test Cases from Bugs

Every fixed bug:
1. Adds a regression test in appropriate category
2. Test name references issue number: `tests/runtime/issue-1234-async-deadlock.nova`

---

## 7. Reference Test Programs

These should always work end-to-end. Run all on every release.

```
examples/01-hello-world      println
examples/02-fibonacci         recursion
examples/03-http-server       std/http
examples/04-todo-cli          fs + collections
examples/05-json-api          json + http server
examples/06-counter-spa       WASM + signals
examples/07-fullstack-blog    SSR + DB
```

---

## 8. Comparing Against Other Languages

Comparison benchmarks (informational, not gating):
```
benchmarks/
├── http-hello-world/
│   ├── nova.nova
│   ├── bun.ts
│   ├── go.go
│   ├── rust-axum.rs
│   └── README.md (results table)
├── json-parse-1mb/
└── fibonacci-40/
```

Run on standardized hardware (GitHub-hosted runners + dedicated benchmarking VM).
