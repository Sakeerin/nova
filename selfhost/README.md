# `selfhost/`: Nova's compiler, written in Nova

This package is **N**, the self-hosted Nova compiler. It reads a Nova
program and writes its LLVM IR (`.ll`), the same text the Rust compiler
writes. Then, like `nova build --release`, it runs `clang` and `cc` on
that `.ll` to leave a native binary.

N is not shipped. The release ships the Rust `nova`, and N stays in the
repository, unpublished; no user is meant to run it as a tool (phase
plan decision 5, `docs/phase-5-plan.md`). It is Phase 5's proof that
Nova can compile itself, and the tests below hold it to the Rust
compiler, byte for byte.

## The names

- **S0:** the Rust compiler, `nova`, built by cargo from `crates/`.
- **N:** this package, `selfhost/src/main.nova` and the modules it
  imports. Its own tests sit in the `*_tests.nova` files.
- **S1:** N built by S0. The tests build it with `nova build` in this
  directory (Cranelift). The bootstrap builds it with `nova build
  --release` (`clang`).
- **S2:** N built by S1's own `build`.
- **S3:** N built by S2's own `build`.

## Its commands

Each command prints what one of S0's commands prints, byte for byte. The
tests compare the two. These are N's usage lines, as it prints them on a
command line it does not take, with exit code 2:

```
usage: selfhost tokens|ast FILE...
usage: selfhost resolve --std DIR [--target native|wasm] [--test] (--std-part | FILE...)
usage: selfhost hir --std DIR [--target native|wasm] [--test] (--std-part | FILE...)
usage: selfhost mir --std DIR [--before-async | --ll] FILE...
usage: selfhost build FILE [-o OUT] [--release] --std DIR
usage: selfhost check FILE --std DIR
```

| N | S0 | What it prints |
|---|---|---|
| `tokens` | `nova parse --dump-tokens` | each file's tokens (ADR 0051) |
| `ast` | `nova parse --dump-ast` | each file's AST (ADR 0052) |
| `resolve` | `nova check --dump-resolution` | names resolved, per root (ADRs 0053, 0054) |
| `hir` | `nova check --dump-hir` | items, then the checked bodies (ADRs 0055, 0056) |
| `mir` | `nova check --dump-mir` | native MIR, after the async transform or `--before-async` before it; with `--ll`, each lowered root's `.ll` (ADRs 0057, 0058) |
| `build` | `nova build --release FILE -o OUT` | `built OUT`, leaving a binary (ADR 0059) |
| `check` | `nova check FILE` | each diagnostic's code and locus, then `ok: FILE` (ADR 0059) |

- **std is read only from `--std DIR`,** never from beside the
  executable or from `$NOVA_HOME`.
- **`build` works as S0 does:**
  - it writes `OUT.nova.ll` beside OUT;
  - it runs `clang -O2 -c` on it (`$NOVA_CLANG`, or `clang`), then `cc`
    with S0's line (`$CC`, or `cc`);
  - it finds the runtime in `$NOVA_RUNTIME_LIB`, or else beside the
    executable or up to two directories above it;
  - it removes the `.ll` and the object once the build succeeds;
  - without `--release` it builds the same way.

  It refuses Windows. Its linking on macOS has not yet run.

## S1, S2 and S3 by hand

The chain needs:
- `clang` 18.1.3 (`clang-18` on Ubuntu 24.04);
- `cc`;
- a release build of the runtime.

A checkout's `nova` carries no runtime unless it is built with
`NOVA_EMBED_RUNTIME=1`. Point `NOVA_RUNTIME_LIB` at a release
`libnova_runtime.a`, and give all three stages **the same file**.

Use **one `-o` basename for every stage**, in different directories. The
`.ll`'s name, `selfhost.nova.ll` here, reaches the binary.

From the repository's root:

```sh
export NOVA_CLANG=clang-18 NOVA_RUNTIME_LIB=/path/to/release/libnova_runtime.a
mkdir -p /tmp/chain/s1 /tmp/chain/s2 /tmp/chain/s3
nova build --release selfhost/src/main.nova -o /tmp/chain/s1/selfhost
/tmp/chain/s1/selfhost build selfhost/src/main.nova -o /tmp/chain/s2/selfhost --release --std std
/tmp/chain/s2/selfhost build selfhost/src/main.nova -o /tmp/chain/s3/selfhost --release --std std
cmp /tmp/chain/s1/selfhost /tmp/chain/s2/selfhost && cmp /tmp/chain/s2/selfhost /tmp/chain/s3/selfhost
```

Each stage takes about 15 s, and `clang` is most of that. The three
binaries are the same, byte for byte.

**What breaks the equality:**
- another basename;
- another runtime archive;
- another `clang`;
- another link driver (`CC=clang-18`, for instance).

**What does not:** paths, the working directory, `TMPDIR`, the locale,
the time and the environment. ADR 0060 records the probes.

To keep a stage's `.ll`, rerun its command with `NOVA_CLANG` (and, for
S0, `NOVA_LLC`) naming a file that does not exist. The build then stops
at the `clang` probe, exits 1 and leaves `OUT.nova.ll`.

## How the tests run

`cargo test --workspace` skips `crates/nova-cli/tests/selfhost.rs`,
because its target says `test = false`. CI's Selfhost job runs it on
Linux, macOS and Windows:

```sh
cargo test --workspace --test selfhost
```

- **The differential.** It compares each of N's dumps with S0's, byte
  for byte, with the same exit codes, on:
  - the corpus (every `.nova` file in the repository, N's own included);
  - the fixed cases under `crates/nova-cli/tests/selfhost/cases/`;
  - the fuzzers' inputs, on fixed seeds;
  - programs nested 128 levels deep.

  N is built once by S0's Cranelift backend into a fixed temporary
  directory, so this needs no `clang`. A difference is reported by file
  and line, with its place (a MIR function and block, or a `.ll` line,
  `define` and block), and the commands that reproduce it.
  `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer fuzz locally.
- **N's own `@test`s** run through `the_selfhost_tests_pass`, which runs
  `nova test` in this directory. Every `@test` must pass, so a run of no
  test at all fails.
- **The tests that need `clang`** are `#[ignore]`d with the reason
  "needs clang; the Selfhost job's Linux step runs it". CI names each
  one by `--exact` on a step of the Selfhost job's Linux leg:

  ```sh
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --workspace --test selfhost -- \
      --ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --workspace --test selfhost -- \
      --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
  ```

  - `s1_builds_the_golden_suite` has S1 build the 147 golden programs
    and holds each binary equal to S0 `--release`'s.
  - `n_build_answers_as_s0_does_with_clang` holds N's `build` to S0's
    messages.
  - `the_bootstrap_holds_s1_s2_s3` is the bootstrap gate:
    - S1 by S0 `--release`, S2 by S1 and S3 by S2, in order;
    - N's `.ll` from S1 and S2 equal to S0's;
    - S1 == S2 == S3 (ADR 0060).

  Without `clang` each of these prints that it did not run, and passes.
  Under `NOVA_REQUIRE_CLANG=1` each fails instead.

  `the_ci_step_names_every_clang_test` holds the steps' names to
  exactly these tests: a name that matches nothing would run nothing.
- **Clippy** lints the file by name, because `--all-targets` skips it:

  ```sh
  cargo clippy --all-features --test selfhost -- -D warnings
  ```
- **N's sources are formatted:**

  ```sh
  nova fmt --check std examples selfhost
  ```

## The rule: S0 and N change together

Each stage of N is a port of S0's, and the differential holds the two
equal. So a change to the language, to a dump, to the `.ll` emitter, to
the runtime functions' symbols, signatures or order, or to the runtime's
ABI as the emitter spells it, changes **both compilers in one pull
request** (phase plan decision 6; ADR 0058 §16). A part that N does not
port (`run`, `test`, `fmt`, the LSP, the WASM backend, and WASM and
test-mode code generation) changes in Rust alone.

## The records

- ADR 0048: Phase 5's scope and backend.
- ADR 0049: tuples and patterns.
- ADR 0050: the std a compiler needs.
- ADR 0051: the lexer and the differential.
- ADR 0052: the parser, the AST dump and the depth limit.
- ADR 0053: the loader, the resolver and the resolution dump.
- ADR 0054: packages, the WASM target and test mode in the resolution
  dump.
- ADR 0055: item collection and the HIR dump.
- ADR 0056: the type checker and the full HIR dump.
- ADR 0057: MIR and the MIR dump.
- ADR 0058: the `.ll` emitter and the `.ll` dump.
- ADR 0059: the driver, `build` and `check`.
- ADR 0060: the bootstrap gate and its tolerances.

The ADRs are in `docs/adr/`. Phase 5's plan is `docs/phase-5-plan.md`,
and each sub-phase's spec and plan are under `docs/superpowers/`.
