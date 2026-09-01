# `std/http` Request Parsing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the server half of `std/http` — an HTTP/1.1 request parser and a response serialiser — so Phase 2's examples 03/04/05 become writable and its 10k req/sec gate becomes measurable.

**Architecture:** Parsing is one Rust intrinsic backed by `httparse`, returning a flat table of byte offsets into the caller's own buffer; it copies nothing, allocates nothing on the Rust heap, and holds no state between calls. Everything else is Nova: the accept loop, the `Request`/`Response` records, header materialisation, and response serialisation by string concatenation. Hyper does not drive the server — spec section 2 records the three measured blockers.

**Tech Stack:** Rust (`crates/nova-runtime`), `httparse` 1.10, Nova (`std/http/lib.nova`), the existing `std/net` transport and `std/collections` `Map`.

**Spec:** `docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md`

## Global Constraints

Every task's requirements implicitly include this section.

- `cargo build --locked --workspace` **before** `cargo test`. A test run that follows a failed build proves nothing.
- `cargo test --workspace --no-fail-fast`. `--no-fail-fast` is mandatory.
- **Never pipe cargo output through `head`/`tail` before summing.** There are 44 test targets; sum **every** `test result:` line. Baseline on this branch, measured locally on Windows: **1081 passed / 0 failed / 8 ignored**. A 1076 figure for Windows circulates in this project's notes; a local run of this branch reports 1081, so do not reconcile your sum against 1076. Whether CI's Windows job counts a different population is open and this plan does not rest on it -- **report your own sum and let it stand on its own.**
- No `reason = "..."` in any lint attribute. MSRV is 1.78 (`Cargo.toml`'s `workspace.package.rust-version`) and `reason` postdates it.
- `cargo clippy --all-targets -- -D warnings` must pass on **both** ubuntu and windows.
- `cargo fmt --all -- --check`. Note: `cargo fmt --all` writes LF into this CRLF working copy — after running it, check `git diff --numstat` and confirm the change count matches what you meant.
- The ignored ADR-0010 GC tests stay ignored and untouched.
- **The poll ABI is frozen and no panic may cross a generated poll boundary.** This constraint is **live** in this increment, not inert: the parse intrinsic is reachable from a compiled Nova frame. Task 1 adds the mechanical guard that holds it.
- Every fixture path unique per process.
- Commit messages written to a UTF-8 file and applied with `git commit -F`, **never a heredoc**. Every body ends exactly with the line `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA that is not already an ancestor of `main`.** `2cb3adf` is. Every commit made on this branch — the spec commit included — is **branch-local** until it lands, so none of their SHAs may appear in any tracked file. Refer to those changes by what they did. Get the branch-local roster with `git log --format=%h main..HEAD` rather than from memory, and note that this plan does not quote it: a SHA written down here would itself be a citation.
- **Byte-scan every file you write:** valid UTF-8; no byte below 0x20 outside tab, CR and LF; no 0x7f; and **zero occurrences of backslash-u followed by four hex digits** in tracked markdown — write code points as U+XXXX. Do that scan in Python with the pattern built from `chr(92)`, and assert it against a planted positive first: in a POSIX ERE `\u` degrades to a literal `u`, so a `grep -E` for that pattern matches the word "succeeds".
- **Do not author Nova string escapes through a heredoc.** A quoted bash heredoc consumed a backslash on an earlier branch, turning `"a\"b\\c"` into `a\"b\c` and failing L0001. Use the Write tool or a Python rewrite that asserts a match count.
- **Sentence-shape discipline**, binding on every comment, doc and record: prefer a roster with no count; a corrected number is usually the wrong fix; no ordinals or closed worlds over `std`, the runtime, the workspace or the record set; never claim a test is "the only" thing that catches something — that shape has been measured false repeatedly here. grep is line-oriented, so a miss is **not** evidence of absence: sweep prose with whitespace-tolerant patterns that also normalise `//`, `///` and `>` gutters.

### Known flake, and how to handle it

Roughly one run in four, an async/threading test fails on Windows. It has historically carried `0xc0000005`, but **not always** — a 2026-08-29 instance carried no crash code at all, just an async child exiting non-zero with empty stdout. The cause is not established and every shared-path hypothesis has been eliminated. If you hit it: **re-run, say so in your report, attribute no cause, and fix nothing.** Do not grep for `0xc0000005` as the test of whether it fired.

### Measured Nova language facts

No `?` operator. No turbofish. No `as` cast. No `Int`->`Char`, `String`->number or `Float`->`Int` conversion. No `loop` keyword, but `break`/`continue` work. `match` arms cannot bind `mut`. Tuples are `E0900`. No field privacy. Records are **reference** types. `Int` wraps silently, and `>>` is arithmetic with no `>>>`, so shifts need masking. Type aliases do not exist, but `pub type X = | A | B` declares a **sum type**, and its variants may carry payloads (`| String(String)` in `std/json` proves a variant name may equal a type name). Top-level consts are `const NAME: Type = value` with no `pub`. `[x; n]` is the only runtime-length array allocation and needs a filler. A raw `[T]` has `.len()` and `a[i]` indexing and `a[i] = v` assignment. String interpolation is `"${expr}"`. `\r` **is** a supported string escape (`crates/nova-lexer/src/lib.rs`, the `b'r' => buf.push('\r')` arm). `pub` visibility is enforced across modules (`crates/nova-resolver/src/lib.rs:1292`; tests `importing_a_private_item_is_rejected` and `private_item_is_not_visible_to_other_modules`).

### The CRLF decision, stated once so no task re-decides it

HTTP is CRLF-delimited and this working copy is CRLF. **Every fixture builds its CRLFs from in-Nova `\r\n` escapes inside a `bytes_from_string(...)` call. No tracked fixture file stores a literal CR byte as request data.** Git's line-ending handling therefore cannot rewrite the bytes under test. The paired `.stdout` goldens are ordinary LF text; the harness in `crates/nova-cli/tests/run_tests.rs` already normalises them with `.replace("\r\n", "\n")` before comparing.

### The twelve-site intrinsic checklist

ADR 0018 section 3 ("The seam count, and the trap that matters more than the count") is the authority. Its counting rule: a site is one declaration, `match` arm, array or function body in `crates/` that must change. Doc comments belong to the variant they sit on, array length annotations belong to their array, and tests are coverage rather than seam.

Verified on this tree against `Builtin::IntHashSeed`, the most recently added intrinsic. **These are the twelve sites, which is not the same set as any grep's output** — see the note under the table before you try to confirm the count with a grep:

| # | file | what |
|---|---|---|
| 1 | `crates/nova-resolver/src/lib.rs:768` | `Builtin` variant, with its doc comment above it |
| 2 | `crates/nova-resolver/src/lib.rs:874` | `Builtin::X => "name"` in `name()` — **forced** |
| 3 | `crates/nova-resolver/src/lib.rs:972` | element of `Builtin::STD_ONLY`, whose length annotation moves with it |
| 4 | `crates/nova-typeck/src/check.rs:3973` | the no-hint `match` arm list — **forced** |
| 5 | `crates/nova-typeck/src/check.rs:7274` | the signature table — **forced** |
| 6 | `crates/nova-typeck/src/check.rs:15384` | the description table inside `#[cfg(test)] mod tests` — **forced, and invisible to a plain `cargo check`** |
| 7 | `crates/nova-mir/src/lib.rs:460` | `RtFunc` variant, with its doc comment |
| 8 | `crates/nova-mir/src/lib.rs:556` | `RtFunc::X => "nova_rt_x"` in `symbol()` — **forced** |
| 9 | `crates/nova-mir/src/lib.rs:731` | the MIR signature table — **forced** |
| 10 | `crates/nova-mir/src/lower.rs:746` | `Builtin::X => Lowering::Runtime(RtFunc::X)` — **forced** |
| 11 | `crates/nova-runtime/src/...` | the `extern "C" fn nova_rt_x` definition |
| 12 | `crates/nova-runtime/src/lib.rs:873` | the `symbols()` entry |

`RtFunc::ALL` is **not** a thirteenth site: it is generated by the `rt_funcs!` macro from the same variant list (`crates/nova-mir/src/lib.rs:125-153`).

**No single grep returns exactly the twelve sites, and reaching for one is the trap ADR 0018 warns about.** Measured on this tree:

- `grep -rn 'StrToFloat' crates/ --include=*.rs` returns exactly 10 lines, every one naming a variant. That is ADR 0018's own figure and it still holds — because `StrToFloat` has no lowering test naming it.
- `grep -rn 'IntHashSeed' crates/ --include=*.rs` returns **12**: the same 10 variant-naming sites, plus two lines belonging to its lowering test. Tests are coverage rather than seam under ADR 0018's counting rule, so those two are not sites — the 12 here is a coincidence of arithmetic, not the site count.
- The two remaining sites name the **C symbol** instead of a variant: the `extern "C" fn` definition and the `symbols()` entry. A grep for `nova_rt_<name>` finds both, and also every doc-comment mention of them.

So: 10 variant-naming sites plus 2 C-symbol sites is 12, and that arithmetic is the verification. **Report the roster and say which category each line falls in.** A bare count proves nothing, whichever pattern produced it.

Two facts about site 6 and site 12 that decide how you verify this work:

- **`cargo check --workspace` finds six of the seven forced sites and reports success**, because site 6 lives in a `#[cfg(test)]` module. `cargo check --locked --workspace --all-targets` is mandatory here; a plain green build is not evidence.
- **Site 12 survives every compiler in the pipeline** — Rust's including all test targets, and Nova's — and fails at link time inside the JIT. It is held by `every_rt_func_symbol_is_registered_with_the_jit` in `crates/nova-codegen-cranelift/src/lib.rs`.
- Omitting site 3 (`STD_ONLY`) together with its length annotation compiles clean, but its consequence is loud and immediate: `std/http` is compiled on every `nova` invocation, so the omission yields `error[E0001]: cannot find function 'http_parse_request' in this scope` universally.

---

## File Structure

**Created:**

| file | responsibility |
|---|---|
| `crates/nova-runtime/src/http.rs` | the parse intrinsic, its limit constants, its error-kind constants, its Rust unit tests, and its panic-freedom guard |
| `std/http/lib.nova` | `Method`, `Request`, `Response`, `Limits`, `HttpError`, the offset-table decoder, response serialisation, and `read_request`/`write_response` |
| `tests/runtime/http_offsets.nova` + `.stdout` | pins the exact offset array for a known request, field by field |
| `tests/runtime/http_partial.nova` + `.stdout` | a head split across two reads: status 1 then 0, offsets identical to the unsplit case |
| `tests/runtime/http_malformed.nova` + `.stdout` | one case per error kind, asserting the negative status and that the array has length 1 |
| `tests/runtime/http_limits.nova` + `.stdout` | each limit at its boundary and one past it |
| `tests/runtime/http_serialise.nova` + `.stdout` | parse a request, serialise a response, compare bytes to a golden |
| `tests/runtime/http_keepalive.nova` + `.stdout` | two requests on one connection, both parsed, both ends in Nova |
| `docs/adr/0019-offset-table-intrinsic-boundary.md` | the new ADR spec section 9 requires |

**Modified:** `crates/nova-runtime/Cargo.toml`, `Cargo.lock`, `crates/nova-runtime/src/lib.rs`, `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`, `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`, `crates/nova-mir/tests/lower_tests.rs`, `crates/nova-cli/tests/run_tests.rs`, `nova-spec/20-STDLIB.md`, `nova-spec/00-MASTER-SPEC.md`, `CHANGELOG.md`.

---

## Three rulings this plan makes, and why

The spec is the authority. These resolve places where it under-determines the work; each is recorded here so no task re-litigates it.

**1. `Limits` keeps all three fields, but the two head limits are ceilings the caller may only tighten.** Spec section 4.1 fixes the intrinsic's signature at one argument, so Rust cannot receive a caller's limits; spec section 6 requires the limits be checked in Rust before any Nova allocation. Both hold if the Rust side compiles in the hard wall (`MAX_HEAD_BYTES = 8192`, `MAX_HEADER_COUNT = 100`) and the Nova `Limits` record lets a caller choose a **stricter** value, checked Nova-side. A caller cannot loosen past the Rust ceiling. `max_body_bytes` is purely Nova-side: the intrinsic returns `body_start` and never looks at the body.

**2. `Method`'s catch-all arm is `Unknown(String)`, not the spec's `Other`.** Two reasons. `Other` is already an `IoErrorKind` variant in `std/io`, and every std module is glob-imported into every module; whether two std modules may export the same variant name is **not established on this tree** and this increment is not the place to find out. And a payload lets the arm carry the raw method token, which a payload-less arm discards — `std/json`'s `| String(String)` proves both that payloads work and that a variant name may equal a type name. Measured: no existing std sum-type variant is named `Unknown`.

**3. The offset-table block is allocated `scanned`, exactly as `nova_rt_bytes_to_ints` allocates its own.** An `[Int]` block holds no pointers, so `scan: false` would be defensible — but `bytes_to_ints` and `str_chars` both pass `true`, and Nova's codegen reads every `[Int]` block in that one form. Changing the flag is a separate decision about every array in the language, not this increment's. Do not change it, and do not present the difference as an oversight.

---

## Task 1: The `httparse` dependency and the parse intrinsic's Rust half

**Files:**
- Modify: `crates/nova-runtime/Cargo.toml`
- Modify: `Cargo.lock` (by cargo, never by hand)
- Create: `crates/nova-runtime/src/http.rs`
- Modify: `crates/nova-runtime/src/lib.rs` (add `mod http;` beside `mod net;` at `:62`; add the `symbols()` entry near `:873`)

**Interfaces:**
- Consumes: `crate::NovaStr` (`crates/nova-runtime/src/lib.rs:83`); `crate::bytes::as_bytes(b: *const NovaStr) -> &[u8]` (`crates/nova-runtime/src/bytes.rs:48`, `pub(crate)`); `crate::gc::alloc(size: usize, scan: bool) -> *mut u8` (`crates/nova-runtime/src/gc.rs:158`).
- Produces: `nova_rt_http_parse_request(b: *const NovaStr) -> *mut u8`, a plain `extern "C"` symbol returning a Nova `[Int]` block. Task 2 wires it to a `Builtin`; Task 3 calls it from Nova.

### Context an implementer needs

`httparse` is hyper's own HTTP/1 parser: standalone, zero-copy, no async, no runtime. Measured on 2026-09-01 with `cargo info httparse`: version **1.10.1**, **no dependencies at all**, `rust-version: unknown` (it declares no MSRV), features `default = ["std"]` and `std = []`. crates.io is reachable from this environment.

Its shape: `httparse::Request::new(&mut headers)` over a `[httparse::EMPTY_HEADER; N]` array, then `req.parse(buf)` returning `Result<httparse::Status<usize>, httparse::Error>`. `Status::Complete(n)` gives `n` = bytes consumed by the head, which is exactly `body_start`. `Status::Partial` means read more. After a complete parse, `req.method` and `req.path` are `Some(&str)` and `req.version` is `Some(u8)` (the **minor** version).

**Every slice httparse hands back points into `buf`** — that is what zero-copy means, and it is how offsets are recovered: `part.as_ptr() as usize - buf.as_ptr() as usize`. The code below still range-checks each one and returns an error status if a slice falls outside the buffer, rather than trusting the contract.

**One property to verify rather than assume:** httparse shortens `req.headers` to the parsed prefix after a successful parse, so `req.headers.len()` is the header count. Step 3's test `parses_a_request_with_two_headers` measures that directly. If it turns out false, count the headers by scanning for the first `EMPTY_HEADER` instead, and say so in your report.

### The panic-freedom rule, and what it forbids

`crates/nova-runtime/src/net.rs`'s `no_net_intrinsic_can_panic` scans that file's production text for `.borrow_mut()`, `.borrow()`, `unwrap()`, `.expect(`, `panic!` and `format!`. Step 7 adds the same guard to `http.rs`, so **none of those may appear in this file's production code.** Use `match` and `let ... else` instead. The guard does not scan for `[i]` indexing — it fails open there, exactly as `net.rs`'s own guard documents — so avoid indexing by hand and let the code below's bounded pointer writes and iterator loops carry the work.

- [ ] **Step 1: Add the dependency**

Run:

```bash
cargo add httparse@1.10 --package nova-runtime
```

Then confirm the manifest reads as intended and add the comment explaining why the crate is here. `crates/nova-runtime/Cargo.toml`'s `[dependencies]` block becomes:

```toml
[dependencies]
nova-diagnostics = { path = "../nova-diagnostics" }
anyhow = { workspace = true }
tracing = { workspace = true }

# `http.rs`'s request-head parser. This is hyper's own HTTP/1 parser, taken
# on its own: standalone, zero-copy, no async and no runtime, so it parks no
# task and registers no waker. The design spec's section 2 records why hyper
# itself cannot drive this project's server and why its parsing internals can
# still be used. `httparse` has no dependencies of its own.
httparse = "1.10"
```

- [ ] **Step 2: Verify the lockfile changed only by that addition**

Run:

```bash
git diff --stat Cargo.lock && git diff Cargo.lock
```

Expected: one added `[[package]]` block naming `httparse`, and nothing else. Record the exact version resolved. If any other package moved, **stop and report it** — spec success criterion 6 says `Cargo.lock` changes only by this addition.

- [ ] **Step 3: Write the failing tests**

Create `crates/nova-runtime/src/http.rs` containing **only** the test module below, so the tests fail to compile against a function that does not exist yet. (Add `mod http;` to `crates/nova-runtime/src/lib.rs` beside `mod net;` at `:62` in this step, or the file is not part of the crate.)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the Nova `[Int]` block the intrinsic returns back into a `Vec`,
    /// through the same layout `nova_rt_bytes_to_ints` writes: word 0 the
    /// element count, element `i` at byte offset `8 + 8*i`.
    fn read_block(block: *mut u8) -> Vec<i64> {
        let words = block as *const i64;
        // SAFETY: `block` is a live Nova `[Int]` block, so word 0 is its count
        // and words `1..=count` are its elements.
        unsafe {
            let n = *words;
            (0..n).map(|i| *words.add(1 + i as usize)).collect()
        }
    }

    fn parse(src: &[u8]) -> Vec<i64> {
        let b = crate::gc_bytes_for_test(src);
        // SAFETY: `b` is a live `NovaStr` for the duration of this call.
        read_block(unsafe { nova_rt_http_parse_request(b) })
    }

    #[test]
    fn parses_a_request_with_two_headers() {
        let src = b"GET /hi HTTP/1.1\r\nHost: a.example\r\nX-K: v\r\n\r\nbody";
        let t = parse(src);
        assert_eq!(t.len(), 8 + 4 * 2, "8 fixed words plus four per header");
        assert_eq!(t[0], 0, "status: complete");
        assert_eq!((t[1], t[2]), (0, 3), "method `GET`");
        assert_eq!((t[3], t[4]), (4, 3), "path `/hi`");
        assert_eq!(t[5], 1, "minor version");
        assert_eq!(t[6], 2, "header count");
        // Header 0: `Host: a.example`
        assert_eq!((t[7], t[8]), (18, 4), "name `Host`");
        assert_eq!((t[9], t[10]), (24, 9), "value `a.example`");
        // Header 1: `X-K: v`
        assert_eq!((t[11], t[12]), (35, 3), "name `X-K`");
        assert_eq!((t[13], t[14]), (40, 1), "value `v`");
        // 45, not 43: the terminating CRLF CRLF begins at 41, so the head
        // ends at 45. Computed from the request bytes, and cross-checked by
        // the next assertion, which fails on any other value.
        assert_eq!(t[15], 45, "body_start, just past the terminating CRLF CRLF");
        assert_eq!(&src[t[15] as usize..], b"body");
    }

    #[test]
    fn a_partial_head_is_status_one_and_carries_nothing_else() {
        let t = parse(b"GET /hi HTTP/1.1\r\nHost: a.exam");
        assert_eq!(t, vec![1], "status 1 alone: the caller must read more");
    }

    #[test]
    fn a_head_over_the_byte_ceiling_is_rejected_before_parsing() {
        let mut src = b"GET /".to_vec();
        src.resize(MAX_HEAD_BYTES + 1, b'x');
        let t = parse(&src);
        assert_eq!(t, vec![-ERR_HEAD_TOO_LARGE]);
    }

    #[test]
    fn a_head_at_the_byte_ceiling_is_not_rejected_for_being_too_large() {
        let mut src = b"GET /".to_vec();
        src.resize(MAX_HEAD_BYTES, b'x');
        let t = parse(&src);
        assert_ne!(
            t,
            vec![-ERR_HEAD_TOO_LARGE],
            "the ceiling is inclusive: exactly MAX_HEAD_BYTES is allowed in"
        );
    }

    #[test]
    fn more_headers_than_the_ceiling_is_its_own_error_kind() {
        let mut src = b"GET / HTTP/1.1\r\n".to_vec();
        for i in 0..=MAX_HEADER_COUNT {
            src.extend_from_slice(b"H");
            src.extend_from_slice(i.to_string().as_bytes());
            src.extend_from_slice(b": v\r\n");
        }
        src.extend_from_slice(b"\r\n");
        assert!(src.len() <= MAX_HEAD_BYTES, "this input tests headers, not bytes");
        let t = parse(&src);
        assert_eq!(t, vec![-ERR_TOO_MANY_HEADERS]);
    }

    #[test]
    fn exactly_the_header_ceiling_parses() {
        let mut src = b"GET / HTTP/1.1\r\n".to_vec();
        for i in 0..MAX_HEADER_COUNT {
            src.extend_from_slice(b"H");
            src.extend_from_slice(i.to_string().as_bytes());
            src.extend_from_slice(b": v\r\n");
        }
        src.extend_from_slice(b"\r\n");
        let t = parse(&src);
        assert_eq!(t[0], 0, "status: complete");
        assert_eq!(t[6], MAX_HEADER_COUNT as i64, "header count at the ceiling");
        assert_eq!(t.len(), 8 + 4 * MAX_HEADER_COUNT);
    }

    #[test]
    fn a_malformed_request_line_is_a_negative_status_of_length_one() {
        let t = parse(b"GET\r\n\r\n");
        assert_eq!(t.len(), 1, "an error carries the status and nothing else");
        assert!(t[0] < 0, "an error status is negative, got {}", t[0]);
    }

    #[test]
    fn an_empty_buffer_is_partial_rather_than_an_error() {
        assert_eq!(parse(b""), vec![1]);
    }

    #[test]
    fn a_request_with_no_headers_still_reports_body_start() {
        let src = b"GET / HTTP/1.1\r\n\r\n";
        let t = parse(src);
        assert_eq!(t[0], 0);
        assert_eq!(t[6], 0, "no headers");
        assert_eq!(t.len(), 8, "8 fixed words and no header quadruples");
        assert_eq!(t[7], src.len() as i64, "body_start is the end of the head");
    }

    #[test]
    fn http_one_zero_reports_minor_version_zero() {
        let t = parse(b"GET / HTTP/1.0\r\n\r\n");
        assert_eq!(t[0], 0);
        assert_eq!(t[5], 0);
    }
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run:

```bash
cargo test --locked --package nova-runtime http:: --no-fail-fast
```

Expected: a compile error naming `nova_rt_http_parse_request`, `MAX_HEAD_BYTES`, `MAX_HEADER_COUNT`, `ERR_HEAD_TOO_LARGE` and `ERR_TOO_MANY_HEADERS` as not found. A compile failure is the expected failing state here; there is no partially-working implementation to get a clean assertion failure from.

- [ ] **Step 5: Write the implementation**

Prepend this to `crates/nova-runtime/src/http.rs`, above the `mod tests` block:

```rust
//! HTTP/1.1 request-head parsing for `std/http`.
//!
//! # One intrinsic, and what it hands back
//!
//! [`nova_rt_http_parse_request`] parses a request head out of a caller-owned
//! buffer and returns a flat table of **byte offsets into that same buffer**.
//! It copies no bytes, allocates nothing on the Rust heap, and keeps no state
//! between calls, so there is nothing here for a caller to release and nothing
//! to leak under load. The design spec
//! (`docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md`,
//! section 4) records the handle-table alternative that was rejected and why.
//!
//! The encoding is a wire contract with `std/http/lib.nova`, and a
//! disagreement about field order yields plausible garbage rather than an
//! error. It is stated in [`nova_rt_http_parse_request`]'s own doc comment and
//! pinned from both sides: from Rust by this module's
//! `parses_a_request_with_two_headers`, and from Nova by
//! `tests/runtime/http_offsets.nova`.
//!
//! # Panic-freedom
//!
//! This intrinsic is reachable from a compiled Nova frame, which has no
//! landing pad to unwind through, so nothing in this file's production code
//! may panic. It is a plain `extern "C"` rather than `extern "C-unwind"`,
//! following `nova_rt_int_hash_seed`: `crate::task::PollFn`'s doc reserves the
//! `"C-unwind"` permission for this runtime's Rust-side entry points rather
//! than for compiled Nova frames. So a panic here would escalate to an abort,
//! and the design's job is that there is no panic to escalate — every fallible
//! step returns a status. `no_http_intrinsic_can_panic`, below, holds that
//! mechanically, the same way `net.rs` and `file.rs` hold their own.

use crate::NovaStr;

/// Largest request head this parser accepts, in bytes, inclusive.
///
/// This is a hard ceiling compiled into the runtime, not a caller's choice:
/// the intrinsic takes one argument and there is nowhere for a caller's limit
/// to arrive. `std/http`'s `Limits` lets a caller pick a **stricter** value,
/// checked on the Nova side before this function is called; nothing can
/// loosen this one.
pub(crate) const MAX_HEAD_BYTES: usize = 8 * 1024;

/// Largest number of headers this parser accepts in one request head.
///
/// A hard ceiling, for the same reason [`MAX_HEAD_BYTES`] is. It also bounds
/// the offset table this function returns and the `Map` the Nova side builds
/// from it.
pub(crate) const MAX_HEADER_COUNT: usize = 100;

// Error kinds. The status word is the **negation** of one of these, so the set
// can grow without changing the table's shape. `std/http`'s `http_error_kind_of`
// is the other half of this numbering, and the two are independent copies —
// the shape that has already produced a miscompile in this project — so
// `tests/runtime/http_malformed.nova` pins them together per kind.
pub(crate) const ERR_HEAD_TOO_LARGE: i64 = 1;
pub(crate) const ERR_TOO_MANY_HEADERS: i64 = 2;
pub(crate) const ERR_BAD_REQUEST_LINE: i64 = 3;
pub(crate) const ERR_BAD_HEADER: i64 = 4;
pub(crate) const ERR_BAD_NEWLINE: i64 = 5;
pub(crate) const ERR_INCOMPLETE_HEAD_FIELDS: i64 = 6;
pub(crate) const ERR_SLICE_OUT_OF_BUFFER: i64 = 7;

/// A one-element `[Int]` block holding `status` and nothing else.
///
/// Every non-zero status returns through here, which is what makes a caller
/// who skips the status check index out of bounds instead of reading a
/// plausible-looking offset. That is deliberate; see the encoding note on
/// [`nova_rt_http_parse_request`].
fn status_only(status: i64) -> *mut u8 {
    let block = crate::gc::alloc(8 + 8, true);
    let words = block as *mut i64;
    // SAFETY: `block` is a fresh allocation of 16 bytes, so words 0 and 1 are
    // in bounds.
    unsafe {
        *words = 1;
        *words.add(1) = status;
    }
    block
}

/// `part`'s byte offset from the start of `base`, or `None` if `part` is not
/// wholly inside `base`.
///
/// Every slice `httparse` returns points into the buffer it was given — that
/// is what its zero-copy contract means — so `None` here is unreachable in
/// practice. It is checked anyway rather than trusted, because the cost is two
/// comparisons and the consequence of a wrong offset is a Nova-side read of
/// the wrong bytes with no error anywhere.
fn offset_within(base: &[u8], part: &[u8]) -> Option<i64> {
    let base_start = base.as_ptr() as usize;
    let base_end = base_start + base.len();
    let part_start = part.as_ptr() as usize;
    let part_end = part_start + part.len();
    if part_start < base_start || part_end > base_end {
        return None;
    }
    match i64::try_from(part_start - base_start) {
        Ok(off) => Some(off),
        Err(_) => None,
    }
}

/// Parse an HTTP/1.1 request head out of `b`, returning a Nova `[Int]` table
/// of byte offsets into `b`.
///
/// # The encoding
///
/// ```text
/// [0]  status
/// [1]  method_start   [2]  method_len
/// [3]  path_start     [4]  path_len
/// [5]  minor_version                     (0 or 1, for HTTP/1.0 vs 1.1)
/// [6]  header_count   = n
/// [7 .. 7+4n)         four Ints per header, in wire order:
///                       name_start, name_len, value_start, value_len
/// [7+4n]              body_start
/// ```
///
/// `status` is `0` for a complete head, `1` for a partial one (the caller must
/// read more bytes and call again), and **negative for an error**, the value
/// being the negation of one of this module's `ERR_*` kinds.
///
/// **When `status` is not `0` the array has length 1** and carries nothing
/// else, so a caller that forgets to check `status` indexes out of bounds
/// rather than reading a plausible-looking offset.
///
/// All offsets are byte offsets from the start of `b`. `body_start` is the
/// offset just past the terminating CRLF CRLF. The body's *length* is not in
/// the table: it comes from `Content-Length`, a header the caller must read
/// and validate anyway.
///
/// # Safety
/// `b` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_http_parse_request(b: *const NovaStr) -> *mut u8 {
    // SAFETY: forwarding this function's own contract.
    let buf = unsafe { crate::bytes::as_bytes(b) };

    if buf.len() > MAX_HEAD_BYTES {
        return status_only(-ERR_HEAD_TOO_LARGE);
    }

    let mut storage = [httparse::EMPTY_HEADER; MAX_HEADER_COUNT];
    let mut req = httparse::Request::new(&mut storage);
    let consumed = match req.parse(buf) {
        Ok(httparse::Status::Complete(n)) => n,
        Ok(httparse::Status::Partial) => return status_only(1),
        Err(httparse::Error::TooManyHeaders) => return status_only(-ERR_TOO_MANY_HEADERS),
        Err(httparse::Error::NewLine) => return status_only(-ERR_BAD_NEWLINE),
        Err(httparse::Error::HeaderName) | Err(httparse::Error::HeaderValue) => {
            return status_only(-ERR_BAD_HEADER)
        }
        // `Status`, `Token`, `Version` and anything a later `httparse` adds:
        // all of them are a malformed request line.
        Err(_) => return status_only(-ERR_BAD_REQUEST_LINE),
    };

    let (Some(method), Some(path), Some(minor)) = (req.method, req.path, req.version) else {
        return status_only(-ERR_INCOMPLETE_HEAD_FIELDS);
    };

    let headers = &req.headers[..];
    let n = headers.len();
    if n > MAX_HEADER_COUNT {
        return status_only(-ERR_TOO_MANY_HEADERS);
    }

    // Two passes, deliberately. Pass one validates every offset while nothing
    // has been allocated, so a rejected parse cannot abandon a half-written
    // block to the collector. Pass two allocates once and writes. Over at most
    // MAX_HEADER_COUNT headers the second walk costs nothing worth trading the
    // property for.
    let Some(method_start) = offset_within(buf, method.as_bytes()) else {
        return status_only(-ERR_SLICE_OUT_OF_BUFFER);
    };
    let Some(path_start) = offset_within(buf, path.as_bytes()) else {
        return status_only(-ERR_SLICE_OUT_OF_BUFFER);
    };
    for h in headers {
        if offset_within(buf, h.name.as_bytes()).is_none()
            || offset_within(buf, h.value).is_none()
        {
            return status_only(-ERR_SLICE_OUT_OF_BUFFER);
        }
    }

    let count = 8 + 4 * n;
    let block = crate::gc::alloc(8 + 8 * count, true);
    let words = block as *mut i64;
    // SAFETY: `block` is a fresh allocation of `8 + 8*count` bytes, so words
    // `0..=count` are in bounds. Every index written below is `< count`.
    unsafe {
        *words = count as i64;
        *words.add(1) = 0;
        *words.add(2) = method_start;
        *words.add(3) = method.len() as i64;
        *words.add(4) = path_start;
        *words.add(5) = path.len() as i64;
        *words.add(6) = i64::from(minor);
        *words.add(7) = n as i64;
        for (i, h) in headers.iter().enumerate() {
            // 7 + 4*i, not 8 + 4*i: header `i`'s `name_start` is ELEMENT
            // 7 + 4*i, and the writes below add the 1-word count header.
            // An earlier draft of this plan had 8 here, which wrote
            // `name_start` into `name_len`'s slot and pushed `value_len`
            // into `body_start`'s. Task 1's own tests caught it.
            let at = 7 + 4 * i;
            let (Some(name_start), Some(value_start)) = (
                offset_within(buf, h.name.as_bytes()),
                offset_within(buf, h.value),
            ) else {
                // Unreachable: pass one checked every header. Writing the
                // error status into the block already allocated keeps this
                // arm total without a second allocation path.
                *words = 1;
                *words.add(1) = -ERR_SLICE_OUT_OF_BUFFER;
                return block;
            };
            *words.add(1 + at) = name_start;
            *words.add(2 + at) = h.name.len() as i64;
            *words.add(3 + at) = value_start;
            *words.add(4 + at) = h.value.len() as i64;
        }
        *words.add(1 + 8 + 4 * n - 1) = consumed as i64;
    }
    block
}
```

Note the last write: `body_start` lives at element index `7 + 4n`, which is word `1 + 7 + 4n` of the block. `1 + 8 + 4*n - 1` is that same word, written in the form that makes the `8 + 4n` element count visible. If you find that expression harder to read than `1 + 7 + 4 * n`, use the latter — they are the same word and the tests pin the result either way.

- [ ] **Step 6: Run the tests to verify they pass**

Run:

```bash
cargo build --locked --workspace && cargo test --locked --package nova-runtime http:: --no-fail-fast
```

Expected: every test in Step 3 passes. If `parses_a_request_with_two_headers` fails on the header count, read the "one property to verify rather than assume" note above and report what you measured.

- [ ] **Step 7: Add the panic-freedom guard**

Append this test to `http.rs`'s `mod tests`, and register the symbol. In `crates/nova-runtime/src/lib.rs`'s `symbols()` (beside the `nova_rt_int_hash_seed` entry at `:873`):

```rust
        (
            "nova_rt_http_parse_request",
            http::nova_rt_http_parse_request as *const u8,
        ),
```

And the guard:

```rust
    /// This module's own panic-freedom claim, pinned the same mechanical way
    /// `net.rs`'s `no_net_intrinsic_can_panic` and this crate's sibling guards
    /// pin theirs: nothing in this file's production code may panic, since the
    /// intrinsic here is reachable from a compiled Nova frame with no landing
    /// pad to unwind through.
    ///
    /// Scans only the part of this file before its own `mod tests` block, for
    /// the same reason and with the same ceiling those guards document: the
    /// *first* occurrence of the split literal is the real boundary, and this
    /// fails open rather than distinguishing a safe `[i]` from a dangerous one.
    #[test]
    fn no_http_intrinsic_can_panic() {
        let source = include_str!("http.rs");
        let production = source.split("mod tests {").next().unwrap_or(source);
        for needle in [
            ".borrow_mut()",
            ".borrow()",
            "unwrap()",
            ".expect(",
            "panic!",
            "format!",
        ] {
            assert!(
                !production.contains(needle),
                "a std/http intrinsic must not panic: `{needle}` found in this \
                 file's production code, which is reachable from a compiled \
                 Nova frame with no landing pad to unwind through"
            );
        }
    }
```

- [ ] **Step 8: Run the full suite**

Run:

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum **every** `test result:` line across all 44 targets. Expected: 1081 + the tests added here, 0 failed, 8 ignored. Report the sum, not a spot check.

- [ ] **Step 9: Mutation 2 — drop the header-count limit**

Delete the `Err(httparse::Error::TooManyHeaders) => ...` arm's distinct status (fold it into the `Err(_)` catch-all) **and** delete the `if n > MAX_HEADER_COUNT` check. Run:

```bash
cargo test --locked --package nova-runtime http:: --no-fail-fast
```

`more_headers_than_the_ceiling_is_its_own_error_kind` must fail. **Report what actually happened**, including which other tests moved. Then revert the mutation and re-run to confirm green.

- [ ] **Step 10: Mutation 3 — return status 0 on partial input**

Change `Ok(httparse::Status::Partial) => return status_only(1)` to `return status_only(0)`. Run the same command. `a_partial_head_is_status_one_and_carries_nothing_else` and `an_empty_buffer_is_partial_rather_than_an_error` must fail. **Report the observed failure mode rather than the predicted one** — a length-1 block carrying status 0 is exactly the out-of-bounds shape the encoding is designed to produce, so downstream the failure may arrive as an abort rather than an assertion. Revert and re-run.

- [ ] **Step 11: Lint and format**

Run:

```bash
cargo clippy --locked --workspace --all-targets -- -D warnings && cargo fmt --all -- --check
```

If `cargo fmt --all -- --check` fails, run `cargo fmt --all`, then `git diff --numstat` and confirm the change count matches what you meant — `cargo fmt` writes LF into this CRLF working copy.

- [ ] **Step 12: Commit**

Write the message to a UTF-8 file and apply it with `git commit -F`. Never a heredoc.

```bash
git add crates/nova-runtime/Cargo.toml Cargo.lock crates/nova-runtime/src/http.rs crates/nova-runtime/src/lib.rs
git commit -F <path-to-message-file>
```

Subject: `feat(runtime): parse an HTTP/1.1 request head into an offset table`. The body states what the intrinsic returns, that it copies and retains nothing, that `httparse` arrived as a dependency with no dependencies of its own, and what mutations 2 and 3 actually did. It ends exactly with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Task 2: The twelve seam sites, so Nova can name the intrinsic

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs` (variant with doc; `name()`; `STD_ONLY` element and its length)
- Modify: `crates/nova-typeck/src/check.rs` (the no-hint arm list; the signature table; the `#[cfg(test)]` description table)
- Modify: `crates/nova-mir/src/lib.rs` (`RtFunc` variant with doc; `symbol()`; the MIR signature table)
- Modify: `crates/nova-mir/src/lower.rs` (the `Lowering::Runtime` arm)
- Modify: `crates/nova-mir/tests/lower_tests.rs` (a lowering test)

**Interfaces:**
- Consumes: `nova_rt_http_parse_request`, from Task 1.
- Produces: `Builtin::HttpParseRequest`, named `http_parse_request` in Nova source, typed `(Bytes) -> [Int]`, `STD_ONLY`. Task 3 calls it from `std/http/lib.nova`.

### How to find the work rather than trust this list

Add **only** the two enum variants — `Builtin::HttpParseRequest` in `crates/nova-resolver/src/lib.rs` and `RtFunc::HttpParseRequest` in `crates/nova-mir/src/lib.rs` — then run:

```bash
cargo check --locked --workspace --all-targets
```

Every forced site comes back as `error[E0004]: non-exhaustive patterns`. **They do not all appear in one build.** `nova-resolver`'s name table fires alone on the first pass, because cargo cannot compile the downstream crates until the resolver builds; the rest appear together on the second. Fixing the first error and stopping means having seen one seventh of the work. Keep running the command until it is clean, then check the unforced sites in the plan header's table by hand.

- [ ] **Step 1: Add the resolver variant and its doc comment**

In `crates/nova-resolver/src/lib.rs`, beside `IntHashSeed` at `:768`:

```rust
    /// `http_parse_request(buf: Bytes) -> [Int]` — an HTTP/1.1 request head
    /// parsed into a flat table of byte offsets into `buf` itself. Copies
    /// nothing, allocates nothing on the Rust heap, and holds no state between
    /// calls, so `std/http` has nothing to release and nothing to leak per
    /// request.
    ///
    /// Element 0 is a status: `0` complete, `1` partial (read more and call
    /// again), negative for an error. **On any non-zero status the array has
    /// length 1**, so a caller who skips the status check indexes out of
    /// bounds rather than reading a plausible-looking offset. The full
    /// encoding lives on `nova_rt_http_parse_request` in
    /// `crates/nova-runtime/src/http.rs`, which is the half that writes it.
    /// Std-only.
    HttpParseRequest,
```

- [ ] **Step 2: Add the MIR variant and its doc comment**

In `crates/nova-mir/src/lib.rs`, beside `IntHashSeed` at `:460`:

```rust
    /// `(bytes) -> ptr` — a Nova `[Int]` offset table for an HTTP/1.1 request
    /// head. Element 0 is a status; a non-zero status yields a length-1 array.
    HttpParseRequest,
```

- [ ] **Step 3: Run the check and let the compiler name the rest**

Run:

```bash
cargo check --locked --workspace --all-targets
```

Expected on the first pass: `error[E0004]` in `crates/nova-resolver/src/lib.rs`'s `name()`. Fill it in:

```rust
            Builtin::HttpParseRequest => "http_parse_request",
```

Re-run. Expected on the second pass: `E0004` in `crates/nova-typeck/src/check.rs` (three sites), `crates/nova-mir/src/lib.rs` (two sites) and `crates/nova-mir/src/lower.rs` (one site). Fill each in:

`crates/nova-typeck/src/check.rs`, the no-hint arm list at `:3973` — add to the chain that ends `=> ""`:

```rust
            | Builtin::HttpParseRequest
```

`crates/nova-typeck/src/check.rs`, the signature table at `:7274`:

```rust
        Builtin::HttpParseRequest => (vec![Ty::Bytes], Ty::Array(Box::new(Ty::Int))),
```

`crates/nova-typeck/src/check.rs`, the description table inside `#[cfg(test)] mod tests` at `:15384`:

```rust
                Builtin::HttpParseRequest => (
                    (vec![Ty::Bytes], Ty::Array(Box::new(Ty::Int))),
                    "`http_parse_request(buf)` in `std/http`'s `parse_offsets`",
                ),
```

`crates/nova-mir/src/lib.rs`, `symbol()` at `:556`:

```rust
            RtFunc::HttpParseRequest => "nova_rt_http_parse_request",
```

`crates/nova-mir/src/lib.rs`, the MIR signature table at `:731`:

```rust
            RtFunc::HttpParseRequest => (vec![MirTy::Ptr], MirTy::Ptr),
```

`crates/nova-mir/src/lower.rs` at `:746`:

```rust
                    Builtin::HttpParseRequest => Lowering::Runtime(RtFunc::HttpParseRequest),
```

Keep re-running until `cargo check --locked --workspace --all-targets` is clean.

- [ ] **Step 4: Add the two unforced sites**

`crates/nova-resolver/src/lib.rs`'s `STD_ONLY` at `:907` — the length annotation goes `[Builtin; 70]` to `[Builtin; 71]`, and the element joins the array beside `Builtin::IntHashSeed` at `:972`:

```rust
        Builtin::HttpParseRequest,
```

Site 12 (`symbols()`) was already added in Task 1, Step 7. Confirm it is there:

```bash
grep -n 'nova_rt_http_parse_request' crates/nova-runtime/src/lib.rs
```

- [ ] **Step 5: Write the lowering test**

`crates/nova-mir/tests/lower_tests.rs` already pins lowering per builtin — see the test near `:968` that asserts `vec![RtFunc::IntHashSeed]`. Add one in the same shape. Read that test first and match its helper and its assertion style; the point is that a Nova call to `http_parse_request` reaches `RtFunc::HttpParseRequest` exactly once, and the existing test is the template for how this file expresses that.

- [ ] **Step 6: Run the guard tests explicitly**

Run:

```bash
cargo test --locked --workspace --no-fail-fast
```

Three tests must pass and are the ones this task is really about:

- `every_rt_func_symbol_is_registered_with_the_jit` (`crates/nova-codegen-cranelift/src/lib.rs`) — holds site 12, which every compiler in the pipeline lets through.
- `no_std_only_builtin_is_a_reserved_word` (`crates/nova-resolver/src/lib.rs`) — a loop over `STD_ONLY`, so the new entry is covered the moment it joins.
- `std_only_builtins_are_visible_inside_std_modules` — the other half: the name resolves inside every std module.

Sum every `test result:` line. Expected: Task 1's total plus the lowering test, 0 failed, 8 ignored.

- [ ] **Step 7: Verify the checklist by hand and report the roster**

Run:

```bash
grep -rn 'HttpParseRequest\|http_parse_request' crates/ --include=*.rs
```

**That alternation matches far more than the sites**, and deliberately so: it catches doc-comment prose in the resolver and in `nova-runtime/src/http.rs`, plus the new lowering test's own text. Expect roughly two dozen lines, not twelve.

Sort the roster into the three categories and report it that way:

- **10 seam sites naming a variant** — `grep -rn 'HttpParseRequest' crates/ --include=*.rs` isolates these, together with the lowering test's own lines, which are coverage rather than seam under ADR 0018's counting rule.
- **2 seam sites naming the C symbol** — the `extern "C" fn` definition and the `symbols()` entry, both shipped by the previous task.
- **everything else** — prose and test text, which are not sites.

10 plus 2 is the twelve, and that arithmetic is the verification. **Do not report a count without the roster, and do not treat a count alone as evidence** — a grep that counts occurrences rather than sites is how an earlier draft of the design spec said `std/net` spent 24 builtins when it spent eight, and an earlier draft of THIS step made the same mistake by predicting twelve lines from a pattern that returns about twenty-seven.

- [ ] **Step 8: Lint, format, and commit**

```bash
cargo clippy --locked --workspace --all-targets -- -D warnings && cargo fmt --all -- --check
```

Then:

```bash
git add crates/nova-resolver crates/nova-typeck crates/nova-mir
git commit -F <path-to-message-file>
```

Subject: `feat(compiler): seat http_parse_request across the intrinsic seam`. The body names each of the twelve sites by file, says which were compiler-forced and which were not, and records that reaching the forced site in `nova-typeck`'s `#[cfg(test)]` module needed `--all-targets`. It ends exactly with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Task 3: `std/http`'s types, and decoding the offset table

**Files:**
- Create: `std/http/lib.nova`
- Modify: `crates/nova-resolver/src/lib.rs` (`STD_MODULES` at `:1484`, `[(&str, &str); 13]` to `; 14`, plus the entry)
- Create: `tests/runtime/http_offsets.nova`, `tests/runtime/http_offsets.stdout`
- Create: `tests/runtime/http_partial.nova`, `tests/runtime/http_partial.stdout`
- Create: `tests/runtime/http_malformed.nova`, `tests/runtime/http_malformed.stdout`
- Create: `tests/runtime/http_limits.nova`, `tests/runtime/http_limits.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs` (four fixture tests)

**Interfaces:**
- Consumes: `http_parse_request(buf: Bytes) -> [Int]` from Task 2. From `std/bytes`: `Bytes::len`, `Bytes::slice`, `Bytes::concat`, `bytes_from_string`, `Bytes::to_string`. From `std/strings`: `String::to_lower`, `String::trim`. From `std/collections`: `Map`, `Vec`. From `std/core`: `Option`, `Result`, `Default`.
- Produces: `pub fn parse_offsets(buf: Bytes) -> [Int]`; `pub type Method`; `pub type HttpErrorKind`; `pub record HttpError`; `pub record Limits` and `impl Default for Limits`; `pub record Request`; `pub fn parse_request_head(buf: Bytes, limits: Limits) -> Result<Option<Request>, HttpError>`. Task 4 adds `Response`; Task 5 adds `read_request`.

### Why `parse_offsets` is `pub`

`http_parse_request` is `STD_ONLY`, so no fixture can call it directly. `std/http` exposes a thin `pub` wrapper so `tests/runtime/http_offsets.nova` can pin the raw array field by field — the same pattern `std/bytes` already uses for `pub fn bytes_from_ints(ints: [Int]) -> Bytes { bytes_from_ints_intrinsic(ints) }`. Without a fixture at that level, a reordering of the encoding is invisible from Nova.

### On lower-casing header names

`String::to_lower` calls `str_to_lower`, which is documented as **full Unicode** lowercase. That is not a hazard here and the reason is worth stating in the code: `httparse` validates a header name as an HTTP token, which is ASCII-only, so `to_lower` sees only ASCII and the Unicode/ASCII distinction has nothing to bite on. Say that where the call is, rather than leaving a reader to wonder.

- [ ] **Step 1: Write the failing fixture for the exact offset array**

Create `tests/runtime/http_offsets.nova`. Use the **Write tool** — the `\r\n` escapes below must survive verbatim, and a quoted bash heredoc has consumed a backslash on this project before.

```nova
// Pins the exact offset array `http_parse_request` returns, field by field.
//
// This is a wire contract between `crates/nova-runtime/src/http.rs` and
// `std/http/lib.nova`, and a disagreement about field order yields plausible
// garbage rather than an error -- so it is pinned from both sides. The Rust
// side is `parses_a_request_with_two_headers` in that module's own tests; this
// is the Nova side, and it reads the array through `parse_offsets` because
// `http_parse_request` itself is `STD_ONLY`.
//
// The request's CRLFs come from in-Nova `\r\n` escapes rather than from
// literal bytes in this file: HTTP is CRLF-delimited, this working copy is
// CRLF, and byte-exactness here is load-bearing, so git's line-ending
// handling is given nothing to rewrite.
//
// The design spec's mutation 1 -- swapping `name_start` and `value_start` in
// the encoding -- must fail this fixture. If it fails only the round-trip
// fixture, this one is not pinning what it claims.
//
// **The method and the path have deliberately DIFFERENT lengths** -- `POST` is
// four bytes and `/hello` is six. An earlier draft used `GET` and `/hi`, both
// three, which left one transposition invisible: a bug swapping the
// `method_len` and `path_len` writes would have read 3 at both slots and
// passed. Every other field in this table is pairwise distinct here, so that
// pair was the one gap. Keep them different lengths if you ever change this
// request.
//
// This request also differs from the one the Rust-side test in
// `crates/nova-runtime/src/http.rs` uses, which is deliberate rather than
// drift: two sides pinning the same encoding over two different inputs catch
// strictly more than two sides pinning one input twice.

fn main() {
    let req = bytes_from_string("POST /hello HTTP/1.1\r\nHost: a.example\r\nX-K: v\r\n\r\nbody")
    let t = parse_offsets(req)
    println("len ${t.len()}")
    println("status ${t[0]}")
    println("method ${t[1]} ${t[2]}")
    println("path ${t[3]} ${t[4]}")
    println("minor ${t[5]}")
    println("headers ${t[6]}")
    println("h0 name ${t[7]} ${t[8]} value ${t[9]} ${t[10]}")
    println("h1 name ${t[11]} ${t[12]} value ${t[13]} ${t[14]}")
    println("body_start ${t[15]}")
    // The offsets are only meaningful against the caller's own buffer, so
    // read the bytes back out through them rather than trusting the numbers
    // alone: a consistent-but-wrong table would pass the lines above.
    match req.slice(t[1], t[1] + t[2]).to_string() {
        Some(s) => println("method text ${s}")
        None => println("method text <non-utf8>")
    }
    // Read the path back through its OWN offsets. This is the pair the
    // method/path length coincidence used to hide: with both three bytes,
    // transposed length writes read the same number at both slots. Reading
    // the bytes out catches it even if the numbers agree.
    match req.slice(t[3], t[3] + t[4]).to_string() {
        Some(s) => println("path text ${s}")
        None => println("path text <non-utf8>")
    }
    match req.slice(t[7], t[7] + t[8]).to_string() {
        Some(s) => println("h0 name text ${s}")
        None => println("h0 name text <non-utf8>")
    }
    match req.slice(t[9], t[9] + t[10]).to_string() {
        Some(s) => println("h0 value text ${s}")
        None => println("h0 value text <non-utf8>")
    }
    match req.slice(t[15], req.len()).to_string() {
        Some(s) => println("body text ${s}")
        None => println("body text <non-utf8>")
    }
}
```

And `tests/runtime/http_offsets.stdout`:

```text
len 16
status 0
method 0 4
path 5 6
minor 1
headers 2
h0 name 22 4 value 28 9
h1 name 39 3 value 44 1
body_start 49
method text POST
path text /hello
h0 name text Host
h0 value text a.example
body text body
```

- [ ] **Step 2: Register the fixture and run it to verify it fails**

In `crates/nova-cli/tests/run_tests.rs`, beside the other `tests/runtime` entries:

```rust
/// The offset table `http_parse_request` returns, pinned field by field from
/// the Nova side. See `tests/runtime/http_offsets.nova`'s own header for why
/// it is pinned from both sides and what mutation it exists to kill.
#[test]
fn http_offsets_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/http_offsets.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/http_offsets.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

Run:

```bash
cargo build --locked --workspace && cargo test --locked --package nova-cli http_offsets_run --no-fail-fast
```

Expected: FAIL with `error[E0001]: cannot find function 'parse_offsets' in this scope`.

- [ ] **Step 3: Create `std/http/lib.nova` and register it**

Create `std/http/lib.nova`:

```nova
// Nova standard library -- HTTP/1.1, server side.
//
// Compiled as an implicit module and glob-imported into every user module, so
// these names need no `import` (see docs/adr/0004-stdlib-compile-model.md).
// The transport is `std/net`: this module parses and serialises, and never
// opens a socket itself.
//
// **What v1 is, against `nova-spec/20-STDLIB.md` section 6.** That section
// specifies a router (`Server::new().get(path, handler)`) over
// `pub type Handler = async fn(Request) -> Response`. That alias does not
// parse -- measured, `P0001: expected type (in type alias), found async` --
// so v1 ships no router and no client. It loses less than it sounds: handler
// code sits inside the caller's own accept loop, which is already an async
// context, so it may `await` freely. A router would have *added* a constraint.
// See docs/adr/0019-offset-table-intrinsic-boundary.md and that section's own
// dated amendment.
//
// **Parsing is one intrinsic and it returns offsets, not values.**
// `http_parse_request` hands back a flat `[Int]` table of byte offsets into
// the caller's own buffer -- no copies, no Rust-side state, nothing to
// release. `parse_offsets` below is a thin `pub` wrapper over it, the same
// shape `std/bytes`'s `bytes_from_ints` uses, so a fixture can pin the raw
// encoding; ordinary callers want `parse_request_head`.
//
// **Header names are lower-cased and the original casing is not kept.** HTTP
// field names are case-insensitive, so a `Map` keyed on the raw casing would
// answer `get("content-length")` differently depending on what the peer sent.

// The request method. `Unknown` carries the raw token rather than discarding
// it: a request may carry any method a peer invents, and the parser must not
// abort on one it does not know.
//
// Named `Unknown` rather than `nova-spec/20-STDLIB.md` section 6's `Other`,
// deliberately. `Other` is already an `IoErrorKind` variant in `std/io`, every
// std module is glob-imported into every module, and whether two std modules
// may export one variant name is not established on this tree. This increment
// is not the place to find out.
pub type Method =
    | Get
    | Post
    | Put
    | Delete
    | Patch
    | Head
    | Options
    | Unknown(String)

// Why a request head was rejected.
//
// **This type is one half of a wire contract.** The other half is the `ERR_*`
// constants in `crates/nova-runtime/src/http.rs`, which produce these codes.
// The two are independent copies of one numbering -- the shape that has
// already produced a miscompile in this project -- so
// `tests/runtime/http_malformed.nova` pins them together, per kind.
pub type HttpErrorKind =
    | HeadTooLarge
    | TooManyHeaders
    | BadRequestLine
    | BadHeader
    | BadNewline
    | IncompleteHeadFields
    | SliceOutOfBuffer
    | BodyTooLarge
    | Transport

pub record HttpError {
    pub kind: HttpErrorKind
    pub message: String
}

// Map a parse status to its kind. The argument is the **negated** status the
// intrinsic returned, so a status of `-3` arrives here as `3`.
//
// An unrecognised code is `BadRequestLine` rather than a panic: a status this
// function does not know is a runtime bug, and mapping it to a diagnosable
// kind keeps an error flowing instead of ending the process -- the same call
// `std/io`'s `io_error_kind_of` makes for the same reason.
pub fn http_error_kind_of(code: Int) -> HttpErrorKind {
    if code == 1 { return HeadTooLarge }
    if code == 2 { return TooManyHeaders }
    if code == 3 { return BadRequestLine }
    if code == 4 { return BadHeader }
    if code == 5 { return BadNewline }
    if code == 6 { return IncompleteHeadFields }
    if code == 7 { return SliceOutOfBuffer }
    BadRequestLine
}

// Caller-chosen ceilings.
//
// **`max_head_bytes` and `max_header_count` may only tighten, never loosen.**
// The runtime compiles in its own hard wall (8 KiB and 100, in
// `crates/nova-runtime/src/http.rs`) and checks it before allocating anything
// on the Nova side, because the intrinsic takes one argument and there is
// nowhere for a caller's limit to arrive. The two fields here are checked on
// this side, in addition. `max_body_bytes` is this side's alone: the intrinsic
// returns `body_start` and never looks at the body.
pub record Limits {
    pub max_head_bytes: Int
    pub max_header_count: Int
    pub max_body_bytes: Int
}

impl Default for Limits {
    fn default() -> Limits {
        Limits { max_head_bytes: 8192, max_header_count: 100, max_body_bytes: 1048576 }
    }
}

// A parsed request. `body` is the bytes after the head, already bounded by
// `Content-Length` and by `Limits::max_body_bytes`.
pub record Request {
    pub method: Method
    pub path: String
    pub headers: Map<String, String>
    pub body: Bytes
}

// The raw offset table for `buf`, exactly as
// `crates/nova-runtime/src/http.rs` encodes it.
//
// **Element 0 is a status and a caller must read it first.** `0` is a complete
// head, `1` is a partial one (read more bytes and call again), and a negative
// value is the negation of an error kind. On any non-zero status **the array
// has length 1**, so a caller who skips the check indexes out of bounds rather
// than reading a plausible-looking offset. That is deliberate.
//
// Exposed so `tests/runtime/http_offsets.nova` can pin the encoding from this
// side; ordinary callers want `parse_request_head`.
pub fn parse_offsets(buf: Bytes) -> [Int] { http_parse_request(buf) }

// Decode a complete head out of `buf`, or report that more bytes are needed.
//
// `Ok(None)` means the head is incomplete: read more and call again with the
// longer buffer. `Ok(Some(req))` is a complete head, with `body` empty --
// `read_request` fills that in once it knows `Content-Length`.
pub fn parse_request_head(buf: Bytes, limits: Limits) -> Result<Option<Request>, HttpError> {
    if buf.len() > limits.max_head_bytes {
        return Err(HttpError { kind: HeadTooLarge, message: "request head over max_head_bytes" })
    }
    let t = parse_offsets(buf)
    let status = t[0]
    if status == 1 { return Ok(None) }
    if status < 0 {
        let kind = http_error_kind_of(0 - status)
        return Err(HttpError { kind: kind, message: "request head rejected by the parser" })
    }
    let n = t[6]
    if n > limits.max_header_count {
        return Err(HttpError { kind: TooManyHeaders, message: "header count over max_header_count" })
    }
    let method = method_from(text_at(buf, t[1], t[2]))
    let path = text_at(buf, t[3], t[4])
    let mut headers: Map<String, String> = Map::new()
    let mut i = 0
    while i < n {
        let at = 7 + 4 * i
        // Lower-cased because HTTP field names are case-insensitive. Full
        // Unicode lowercasing has nothing to bite on here: `httparse`
        // validates a name as an HTTP token, which is ASCII-only, so this
        // sees only ASCII.
        let name = text_at(buf, t[at], t[at + 1]).to_lower()
        let value = text_at(buf, t[at + 2], t[at + 3])
        headers.insert(name, value)
        i = i + 1
    }
    Ok(Some(Request { method: method, path: path, headers: headers, body: bytes_from_string("") }))
}

// The bytes `buf[start .. start+len]` as text.
//
// **Three of this function's four call sites cannot see invalid UTF-8, and one
// can.** Measured against the locked `httparse` 1.10.1 rather than reasoned
// from its byte maps, which mislead on their own:
//
// - **Method** is matched by `is_method_token` against an ASCII-only
//   `TOKEN_MAP`, and httparse hands it back as `&str`.
// - **Path** looks unsafe from the byte map alone -- `URI_MAP` is
//   `byte_map!(b'!'..=0x7e | 0x80..=0xFF)`, so it accepts the high range -- but
//   `parse_uri` then converts with a CHECKED `str::from_utf8` and returns
//   `Err(Error::Token)` on failure, so a non-UTF-8 path is rejected as a
//   malformed request line and never reaches here. Note that httparse's own
//   SAFETY comment above that conversion claims token bytes are "therefore
//   also utf-8", which its own `URI_MAP` contradicts; the checked conversion is
//   what actually holds.
// - **Header name** is `Header.name: &str`, which httparse documents as valid
//   ASCII-US.
// - **Header value** is `Header.value: &[u8]`, and httparse documents why:
//   "the specification allows for values that may not be" ASCII. This is the
//   one site that can receive legal, undecodable bytes.
//
// So this function stays infallible and is used for the three safe sites. The
// header value goes through a fallible sibling instead, because silently
// substituting the empty string there would discard real request data with no
// signal -- which contradicts the reasoning `http_error_kind_of` above gives
// for mapping an unknown code to a diagnosable kind rather than dropping it.
//
// Do not make the three safe sites fallible for symmetry, and do not collapse
// the two functions into one: the asymmetry is the finding, and flattening it
// loses the information.
fn text_at(buf: Bytes, start: Int, len: Int) -> String {
    match buf.slice(start, start + len).to_string() {
        Some(s) => s
        None => ""
    }
}

fn method_from(token: String) -> Method {
    if token == "GET" { return Get }
    if token == "POST" { return Post }
    if token == "PUT" { return Put }
    if token == "DELETE" { return Delete }
    if token == "PATCH" { return Patch }
    if token == "HEAD" { return Head }
    if token == "OPTIONS" { return Options }
    Unknown(token)
}
```

Then register it. In `crates/nova-resolver/src/lib.rs` at `:1484`, the length annotation goes `[(&str, &str); 13]` to `; 14`, and the entry becomes the array's **last** element.

**Last for readability, not because the order is load-bearing — it is not, and this was measured.** The glob-import pass runs *after* every module's items have been collected: `resolve_program` calls `import_std_module` in a loop over all std modules as its last step (`crates/nova-resolver/src/lib.rs:1479`), and that function's own doc comment records that this "also runs std-into-std", which is what lets `std/collections` use `Option` with no import. So `std/http` sees `Map`, `TcpStream`, `String::to_lower` and the rest regardless of where its entry sits.

Order matters for exactly one thing: `import_std_module` merges with `entry().or_insert()`, so if two std modules exported the same name the earlier entry would win, silently and with no diagnostic. A collision check was run before this plan: every name `std/http` exports was compared against a corpus of every top-level `pub record`/`fn`/`type`/`trait`/`const` and every sum-type variant across `std/*/lib.nova`, and there is none. That is also why `Method`'s catch-all arm is `Unknown` rather than the spec's `Other`, which `std/io` already exports.

No consumer hardcodes the module count — `crates/nova-driver/src/lib.rs:604` and `crates/nova-resolver/src/lib.rs:1297` both derive one `FileId` per entry by iterating, and the resolver test at `:2665` loops `1..=STD_MODULES.len()`. Only the length annotation has to move with the element:

```rust
    ("$std.http", include_str!("../../../std/http/lib.nova")),
```

- [ ] **Step 4: Run the fixture to verify it passes**

```bash
cargo build --locked --workspace && cargo test --locked --package nova-cli http_offsets_run --no-fail-fast
```

Expected: PASS. If the numbers differ from the golden, **do not edit the golden to match** — the golden was computed by hand from the request string and the Rust test asserts the same values independently. A mismatch means one of the three is wrong; find out which.

- [ ] **Step 5: Write the partial-input fixture**

Create `tests/runtime/http_partial.nova` with the Write tool:

```nova
// A head split across two reads: status 1 on the first call, 0 on the second,
// and -- the part that matters -- offsets identical to the unsplit case.
//
// A parser that restarted, or that measured offsets against the second chunk
// rather than the whole buffer, would still return status 0 on the second
// call and would still look correct to a test that only checked the status.
//
// The design spec's mutation 3 -- returning status 0 on partial input -- must
// fail this fixture.

fn main() {
    let whole = bytes_from_string("GET /hi HTTP/1.1\r\nHost: a.example\r\n\r\n")
    let first = whole.slice(0, 20)

    let a = parse_offsets(first)
    println("first status ${a[0]}")
    println("first len ${a.len()}")

    let b = parse_offsets(whole)
    let c = parse_offsets(whole)
    println("second status ${b[0]}")
    println("second len ${b.len()}")
    println("unsplit len ${c.len()}")

    let mut same = true
    let mut i = 0
    while i < b.len() {
        if b[i] != c[i] { same = false }
        i = i + 1
    }
    println("offsets identical ${same}")
    println("body_start ${b[b.len() - 1]}")
}
```

And `tests/runtime/http_partial.stdout`:

```text
first status 1
first len 1
second status 0
second len 12
unsplit len 12
offsets identical true
body_start 37
```

- [ ] **Step 6: Write the malformed-input fixture**

Create `tests/runtime/http_malformed.nova` with the Write tool. It asserts, per case, both the negative status and that the array has length 1 — the length is the half that keeps a caller who skips the status check from reading a plausible offset.

```nova
// One case per error kind the parser can report, asserting both halves of the
// contract: the status is negative, and the array has length 1 so nothing
// plausible-looking is readable behind it.
//
// The kinds are numbered in `crates/nova-runtime/src/http.rs` and re-declared
// in `std/http`'s `http_error_kind_of`. Those are independent copies of one
// numbering, so this fixture is where they are held together.

fn report(label: String, src: String) {
    let t = parse_offsets(bytes_from_string(src))
    println("${label} status ${t[0]} len ${t.len()}")
}

fn main() {
    report("bad-request-line", "GET\r\n\r\n")
    report("bad-version", "GET / HTTP/9.9\r\n\r\n")
    report("bad-header", "GET / HTTP/1.1\r\nBad Header: v\r\n\r\n")
    report("empty-is-partial", "")
}
```

And `tests/runtime/http_malformed.stdout` — **do not guess these numbers.** Run the fixture once with a temporary golden, read the actual output, confirm each status is negative and each length is 1 (except `empty-is-partial`, which is status 1 and length 1), then write the observed values as the golden and say in your report which kind each case produced. Some of these inputs may reach a different `httparse::Error` variant than their label suggests; the fixture's job is to pin what actually happens, and the label should be corrected to match rather than the other way round.

- [ ] **Step 7: Write the limits fixture**

Create `tests/runtime/http_limits.nova` with the Write tool. Each limit gets a case at its boundary and one past it. Build the oversized inputs with `String::repeat` rather than by writing them out.

```nova
// Each limit at its boundary and one past it.
//
// The two head limits are enforced twice, on purpose, and this fixture covers
// both: the runtime compiles in a hard wall (8 KiB and 100) that no caller can
// loosen, and `Limits` lets a caller tighten it on the Nova side. A fixture
// that only exercised the hard wall would leave the Nova-side check unpinned,
// and one that only exercised `Limits` would leave the wall unpinned.

fn head_of(header_count: Int) -> Bytes {
    let mut s = "GET / HTTP/1.1\r\n"
    let mut i = 0
    while i < header_count {
        s = "${s}H${i}: v\r\n"
        i = i + 1
    }
    bytes_from_string("${s}\r\n")
}

fn main() {
    // The runtime's hard header wall: 100 in, 101 out.
    let at = parse_offsets(head_of(100))
    println("100 headers status ${at[0]} count ${at[6]}")
    let past = parse_offsets(head_of(101))
    println("101 headers status ${past[0]} len ${past.len()}")

    // The runtime's hard byte wall: a head over 8192 bytes is rejected before
    // parsing begins.
    let big = bytes_from_string("GET /${"x".repeat(8192)} HTTP/1.1\r\n\r\n")
    let over = parse_offsets(big)
    println("oversize status ${over[0]} len ${over.len()}")

    // A caller's own tighter ceilings, checked on this side.
    //
    // `max_head_bytes` is 16 rather than 32 deliberately: `head_of(2)` is
    // measured at exactly 32 bytes, and the check is `len > max_head_bytes`,
    // so a ceiling of 32 would NOT fire and this case would fall through to
    // the header-count check -- printing the same kind as the case below it
    // and testing nothing of its own.
    let tight = Limits { max_head_bytes: 16, max_header_count: 1, max_body_bytes: 16 }
    match parse_request_head(head_of(2), tight) {
        Ok(_) => println("tight head: unexpectedly accepted")
        Err(e) => println("tight head kind ${kind_name(e.kind)}")
    }
    let roomy = Limits { max_head_bytes: 8192, max_header_count: 1, max_body_bytes: 16 }
    match parse_request_head(head_of(2), roomy) {
        Ok(_) => println("tight headers: unexpectedly accepted")
        Err(e) => println("tight headers kind ${kind_name(e.kind)}")
    }
    match parse_request_head(head_of(1), roomy) {
        Ok(r) => println("at the caller ceiling: accepted")
        Err(e) => println("at the caller ceiling: rejected ${kind_name(e.kind)}")
    }
}

fn kind_name(k: HttpErrorKind) -> String {
    match k {
        HeadTooLarge => "HeadTooLarge"
        TooManyHeaders => "TooManyHeaders"
        BadRequestLine => "BadRequestLine"
        BadHeader => "BadHeader"
        BadNewline => "BadNewline"
        IncompleteHeadFields => "IncompleteHeadFields"
        SliceOutOfBuffer => "SliceOutOfBuffer"
        BodyTooLarge => "BodyTooLarge"
        Transport => "Transport"
    }
}
```

The golden `tests/runtime/http_limits.stdout` follows the same rule as Step 6: run it, read the real output, confirm each line says what the comment claims, then write the observed values. `at the caller ceiling: accepted` is the line that proves the ceiling is inclusive rather than exclusive -- `head_of(1)` is 25 bytes with one header, and `max_header_count: 1` admits it because the check is `>` rather than `>=`. The three `head_of` lengths were measured on this tree: `head_of(1)` 25, `head_of(2)` 32, `head_of(100)` 808, `head_of(101)` 817 -- so the two header-ceiling cases sit far below the 8192 byte wall and genuinely test the header wall rather than it.

Note `match parse_request_head(head_of(1), roomy) { Ok(r) => ... }` binds `r` and does not use it. If that draws a warning or an error, bind `_` instead.

- [ ] **Step 8: Register the three remaining fixtures**

Add `http_partial_run`, `http_malformed_run` and `http_limits_run` to `crates/nova-cli/tests/run_tests.rs`, each in the same shape as `http_offsets_run` from Step 2, each with a doc comment saying what its fixture is for.

- [ ] **Step 9: Run the full suite**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum every `test result:` line. Expected: Task 2's total plus four fixtures, 0 failed, 8 ignored.

- [ ] **Step 10: Mutation 4 — skip lower-casing a header name**

In `std/http/lib.nova`'s `parse_request_head`, drop the `.to_lower()`. Add a case to `http_offsets.nova`, or a dedicated assertion in whichever fixture you find clearest, that inserts a header sent as `Host:` and looks it up as `host` — if no fixture currently does that, **add one before running the mutation**, because a mutation nothing catches is a gap in the tests rather than a property of the code. Run:

```bash
cargo test --locked --package nova-cli http_ --no-fail-fast
```

The case-insensitivity assertion must fail. Report what happened, revert, re-run.

- [ ] **Step 11: Lint, format, byte-scan, and commit**

```bash
cargo clippy --locked --workspace --all-targets -- -D warnings && cargo fmt --all -- --check
```

Byte-scan every file written in this task per the global constraints, including the planted-positive assertion. Then:

```bash
git add std/http crates/nova-resolver/src/lib.rs tests/runtime crates/nova-cli/tests/run_tests.rs
git commit -F <path-to-message-file>
```

Subject: `feat(std/http): decode the offset table into a Request`. The body records that `STD_MODULES` gained an entry, why `parse_offsets` is `pub`, the `Unknown(String)` deviation from the spec's `Other` and its two reasons, the two-sided enforcement of the head limits, and what mutation 4 actually did. It ends exactly with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Task 4: `Response` and serialisation, with no new intrinsic

**Files:**
- Modify: `std/http/lib.nova`
- Create: `tests/runtime/http_serialise.nova`, `tests/runtime/http_serialise.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `Request`, `Method`, `HttpError` and `parse_request_head` from Task 3; `bytes_from_string`, `Bytes::concat`, `Bytes::len` from `std/bytes`.
- Produces: `pub record Response { pub status: Int, pub headers: Map<String, String>, pub body: Bytes }`; `Response::ok(body: Bytes) -> Response`; `Response::text(status: Int, s: String) -> Response`; `Response::not_found() -> Response`; `Response::to_bytes(self) -> Bytes`. Task 5 calls `to_bytes` from `write_response`.

A response is `"HTTP/1.1 200 OK\r\n"` plus headers plus `"\r\n"` plus body, built with string interpolation and `Bytes::concat`. **No intrinsic is added for this** — that is spec section 4's decision, not an oversight.

- [ ] **Step 1: Write the failing round-trip fixture**

Create `tests/runtime/http_serialise.nova` with the Write tool:

```nova
// Round trip: parse a request, build a response, and compare the serialised
// bytes to a golden.
//
// The golden is the response text with its CRLFs written as `\r\n` escapes in
// this file, the same decision every fixture in this group makes: HTTP is
// CRLF-delimited, this working copy is CRLF, and byte-exactness is
// load-bearing here, so git's line-ending handling is given nothing to
// rewrite. The printed output below is ordinary LF text.

fn main() {
    let raw = bytes_from_string("GET /hi HTTP/1.1\r\nHost: a.example\r\nContent-Length: 0\r\n\r\n")
    match parse_request_head(raw, Limits::default()) {
        // Two levels of `match`, not one: `Ok(Some(req))` is
        // `E0900: nested patterns inside variants are not supported yet`.
        // Measured on this tree; no std module uses a nested pattern either.
        Ok(maybe) => match maybe {
            Some(req) => {
                println("path ${req.path}")
                println("host ${header_or(req, "host", "<missing>")}")
                println("content-length ${header_or(req, "content-length", "<missing>")}")
                // Sent as `Host:`, looked up as `host`: HTTP field names are
                // case-insensitive and this module lower-cases on insert.
                let resp = Response::text(200, "hi")
                let wire = resp.to_bytes()
                let expected = bytes_from_string("HTTP/1.1 200 OK\r\ncontent-length: 2\r\ncontent-type: text/plain; charset=utf-8\r\n\r\nhi")
                println("wire matches ${wire.eq(expected)}")
                println("wire len ${wire.len()}")
                let nf = Response::not_found().to_bytes()
                println("not_found len ${nf.len()}")
            }
            None => println("unexpectedly partial")
        }
        Err(e) => println("unexpectedly rejected")
    }
}

fn header_or(req: Request, name: String, fallback: String) -> String {
    match req.headers.get(name) {
        Some(v) => v
        None => fallback
    }
}
```

The golden `tests/runtime/http_serialise.stdout`:

```text
path /hi
host a.example
content-length 0
wire matches true
wire len 81
not_found len 95
```

**The two byte counts were computed from the response bytes, not guessed, and they are still to be confirmed by running.** 81 is the status line (17 bytes), plus the content-length header (19), plus the content-type header (41), plus the blank line that ends the head (2), plus the two-byte body. The fixture's own `expected` string is that same 81 bytes, which is an independent check on the figure. 95 is the same arithmetic with a longer status line (24 bytes, for `404 Not Found`) and a nine-byte body. Both counts are header-order independent, which matters because `Map` iteration order varies per process.

Run the fixture and confirm both. **If a count disagrees, find out which side is wrong before touching the golden** — an earlier draft of this plan carried 85 and 76 here, and a golden edited to match whatever the code emitted would have hidden that rather than caught it. If `wire matches` is `false`, the disagreement is between `to_bytes` and the `expected` string, and one of the two is wrong.

- [ ] **Step 2: Register the fixture and run it to verify it fails**

Add `http_serialise_run` to `crates/nova-cli/tests/run_tests.rs` in the same shape as `http_offsets_run`. Run:

```bash
cargo build --locked --workspace && cargo test --locked --package nova-cli http_serialise_run --no-fail-fast
```

Expected: FAIL, with `Response` not found.

- [ ] **Step 3: Implement `Response`**

Append to `std/http/lib.nova`:

```nova
// A response to serialise onto a connection.
//
// **Header order on the wire is `headers`' iteration order**, which is a
// `Map`'s bucket order and therefore varies per process: `impl Hash for
// String` is seeded per process, which is exactly the property that makes a
// `Map` keyed on attacker-supplied header names resist a precomputed
// collision set. HTTP does not require a header order, so nothing here
// depends on it -- but a test comparing whole-response bytes must build its
// expectation the same way rather than assume an order, and
// `tests/runtime/http_serialise.nova` keeps its response to a single header
// set for that reason.
pub record Response {
    pub status: Int
    pub headers: Map<String, String>
    pub body: Bytes
}

impl Response {
    // `200 OK` carrying `body`, with `content-length` set from it.
    pub fn ok(body: Bytes) -> Response {
        let mut h: Map<String, String> = Map::new()
        h.insert("content-length", "${body.len()}")
        Response { status: 200, headers: h, body: body }
    }

    // `status` carrying `s` as UTF-8 text.
    pub fn text(status: Int, s: String) -> Response {
        let body = bytes_from_string(s)
        let mut h: Map<String, String> = Map::new()
        h.insert("content-length", "${body.len()}")
        h.insert("content-type", "text/plain; charset=utf-8")
        Response { status: status, headers: h, body: body }
    }

    pub fn not_found() -> Response { Response::text(404, "not found") }

    // The response as it goes on the wire.
    //
    // No intrinsic backs this: a response is a status line, a header block and
    // a body, which Nova builds with string interpolation and `Bytes::concat`.
    // That is the design's decision rather than a gap -- see
    // docs/adr/0019-offset-table-intrinsic-boundary.md.
    pub fn to_bytes(self) -> Bytes {
        let mut head = "HTTP/1.1 ${self.status} ${reason_phrase(self.status)}\r\n"
        let names = self.headers.keys()
        let mut i = 0
        while i < names.len() {
            let k = names[i]
            match self.headers.get(k) {
                Some(v) => head = "${head}${k}: ${v}\r\n"
                None => head = head
            }
            i = i + 1
        }
        bytes_from_string("${head}\r\n").concat(self.body)
    }
}

// The reason phrase for a status code. Unknown codes get the class's own
// phrase rather than an empty string: HTTP/1.1 lets a client ignore the
// phrase entirely, so a plausible one costs nothing and reads better in a
// packet capture than a blank.
fn reason_phrase(status: Int) -> String {
    if status == 200 { return "OK" }
    if status == 201 { return "Created" }
    if status == 204 { return "No Content" }
    if status == 400 { return "Bad Request" }
    if status == 404 { return "Not Found" }
    if status == 405 { return "Method Not Allowed" }
    if status == 413 { return "Payload Too Large" }
    if status == 500 { return "Internal Server Error" }
    if status < 200 { return "Informational" }
    if status < 300 { return "Success" }
    if status < 400 { return "Redirection" }
    if status < 500 { return "Client Error" }
    "Server Error"
}
```

- [ ] **Step 4: Run and settle the golden**

```bash
cargo build --locked --workspace && cargo test --locked --package nova-cli http_serialise_run --no-fail-fast
```

Get `wire matches true`, then replace the two byte counts in the golden with the measured values, and re-run to green.

If `wire matches` is `false` because the two headers came out in the other order, that is the per-process `Map` order the `Response` doc comment describes, and the fixture is wrong rather than the code: change the fixture to compare a single-header response, or to check the parts rather than the whole. Say which you did and why.

- [ ] **Step 5: Run the full suite**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum every `test result:` line. Expected: Task 3's total plus one fixture, 0 failed, 8 ignored.

- [ ] **Step 6: Mutation 1 — swap `name_start` and `value_start` in the encoding**

This is the design spec's most important mutation and it is run here because it needs both fixtures to make its claim.

In `crates/nova-runtime/src/http.rs`'s header-writing loop, swap the two writes so `name_start` goes where `value_start` belongs and vice versa. Run:

```bash
cargo build --locked --workspace && cargo test --locked --package nova-cli http_ --no-fail-fast && cargo test --locked --package nova-runtime http:: --no-fail-fast
```

`http_offsets_run` must fail. **If only `http_serialise_run` fails, the offsets fixture is not pinning what it claims** — say so plainly and fix the fixture before reverting. Report which tests failed, in full. Then revert and re-run to green.

- [ ] **Step 7: Lint, format, byte-scan, and commit**

```bash
cargo clippy --locked --workspace --all-targets -- -D warnings && cargo fmt --all -- --check
```

Byte-scan every file written. Then commit with subject `feat(std/http): serialise a Response with no new intrinsic`. The body records that response serialisation needed no intrinsic and why, that header order on the wire follows the per-process `Map` order, and exactly what mutation 1 did. It ends exactly with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Task 5: `read_request` over a connection, and keep-alive

**Files:**
- Modify: `std/http/lib.nova`
- Create: `tests/runtime/http_keepalive.nova`, `tests/runtime/http_keepalive.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `parse_request_head`, `Limits`, `Request`, `HttpError` from Task 3; `Response::to_bytes` from Task 4. From `std/net`: `TcpStream`, `TcpListener`, `bind(addr: String) -> Result<TcpListener, IoError>`, `TcpListener::local_port(self) -> Result<Int, IoError>`, `TcpListener::accept(self) -> Result<TcpStream, IoError>` (async), `TcpStream::read(self, max: Int) -> Future<Result<Bytes, IoError>>`, `TcpStream::write(self, buf: Bytes) -> Future<Result<Int, IoError>>`, `TcpStream::close(self)` (async). From `std/task`: `spawn`, `join`.
- Produces: `pub async fn read_request(conn: TcpStream, limits: Limits) -> Result<Request, HttpError>`; `pub async fn write_response(conn: TcpStream, resp: Response) -> Result<Int, IoError>`.

### Two facts about this runtime that shape the loop

**One socket wait per task, enforced by process abort.** Staging two I/O waits in a single poll ends the process (`stage_park` in `crates/nova-runtime/src/task.rs`). A task parked in `accept` cannot also be reading a connection, so a server needs at least two tasks — one accepting, one per connection — and this language has no `select` or `race`. `spawn` is what makes that possible. `std/net`'s `accept` doc comment states this at length; read it before writing the loop.

**`Write::write` may write fewer bytes than given.** `std/net`'s `write` is one non-blocking attempt, not a `write_all` loop, and it returns the count. `write_response` must loop on the returned count.

**`Bytes::concat` is O(n) per call**, so accumulating a head one read at a time is O(n squared) in the head's length. Bounded by `max_head_bytes` at 8 KiB that is not worth engineering around; say so at the loop rather than leaving a reader to wonder whether it was noticed.

**A pipelined second request is silently consumed and dropped, and the consequence is a deadlock rather than an absence.** `read_request` reads into a local buffer and returns `body.slice(0, want)`, so any bytes it already pulled from the socket past the end of the current request's body are discarded when the function returns. A client that writes request 2 without waiting for response 1 — which HTTP/1.1 permits — has those bytes eaten. The server then waits for a request that has already arrived while the client waits for a response that will never come.

Pipelining is out of scope by the design spec's own section 1, but **"out of scope" and "deadlocks the connection" are different claims, and only the second one is true here.** Requirements:

- Say it plainly at `read_request`, in those terms. Do not write that pipelining is "unsupported" and stop there.
- **State that the keep-alive fixture cannot catch it**, and why: its client `await`s each reply before sending the next request, so it never pipelines. A reader who sees a keep-alive fixture passing should not conclude this case is covered.
- Do **not** fix it by changing `read_request`'s signature to hand leftovers back. That is a spec-level API change and is out of scope for this task.

- [ ] **Step 1: Write the failing keep-alive fixture**

Create `tests/runtime/http_keepalive.nova` with the Write tool. Both ends are Nova, in two tasks of one process — the same shape `tests/runtime/net_listener_accept.nova` uses, which binds `127.0.0.1:0` and passes the kernel's chosen port to a spawned client as an ordinary argument. **Read that fixture first and follow it**; it needs no port file and therefore has no port-file path to collide on.

```nova
// Two requests on one connection, both parsed, both answered -- the property
// the Phase 2 gate depends on, since reconnecting per request would dominate
// a throughput number.
//
// Both ends are Nova, in two tasks of one process, following
// `tests/runtime/net_listener_accept.nova`: bind `127.0.0.1:0`, read the
// kernel's choice back with `local_port`, and hand it to a spawned client as
// an ordinary argument. No port file, so no port-file path to collide on.
//
// The client's requests carry their CRLFs as in-Nova `\r\n` escapes, the same
// decision every fixture in this group makes.

async fn client(port: Int) {
    let s = match connect("127.0.0.1:${port}").await {
        Ok(s) => s
        Err(e) => panic("client connect: ${e.message}")
    }
    send_and_report(s, "GET /one HTTP/1.1\r\nHost: h\r\nContent-Length: 0\r\n\r\n", "first")
    send_and_report(s, "GET /two HTTP/1.1\r\nHost: h\r\nContent-Length: 0\r\n\r\n", "second")
    let _ = s.close().await
}

async fn send_and_report(s: TcpStream, req: String, label: String) {
    let _ = match s.write(bytes_from_string(req)).await {
        Ok(n) => n
        Err(e) => panic("client write: ${e.message}")
    }
    let reply = match s.read(512).await {
        Ok(b) => b
        Err(e) => panic("client read: ${e.message}")
    }
    match reply.to_string() {
        Some(t) => println("client ${label}: got ${t.len()} bytes, 200 ${t.contains("200 OK")}")
        None => println("client ${label}: <non-utf8>")
    }
}

async fn main() {
    let l = match bind("127.0.0.1:0") {
        Ok(l) => l
        Err(e) => panic("bind: ${e.message}")
    }
    let port = match l.local_port() {
        Ok(p) => p
        Err(e) => panic("local_port: ${e.message}")
    }
    let h = spawn(client(port))

    let conn = match l.accept().await {
        Ok(c) => c
        Err(e) => panic("accept: ${e.message}")
    }
    // Keep-alive: two requests, one connection, one accept.
    let mut i = 0
    while i < 2 {
        match read_request(conn, Limits::default()).await {
            Ok(req) => {
                println("server: ${req.path} host ${header_or(req, "host", "<missing>")}")
                let _ = write_response(conn, Response::text(200, "ok ${req.path}")).await
            }
            Err(e) => println("server: rejected")
        }
        i = i + 1
    }
    let _ = conn.close().await

    h.join().await
    let _ = l.close().await
    println("server: done")
}

fn header_or(req: Request, name: String, fallback: String) -> String {
    match req.headers.get(name) {
        Some(v) => v
        None => fallback
    }
}
```

The golden `tests/runtime/http_keepalive.stdout` must be written **after** measuring, never guessed, because the interleaving of the client's and server's lines is what a golden pins and a fixture whose golden depends on a scheduling race is worse than no fixture.

**This exact shape was probed before the plan reached you, and it is stable.** A stand-in version — real loopback sockets, two Nova tasks, one accepted connection, two request/response round trips, `write_all` looping on the returned count — produced byte-identical output on five consecutive runs, in the order: server read, server wrote, client report, twice, then the closing line. That is what a single-threaded cooperative executor with two wake sources should do, and it is what it did.

Five runs is evidence, not proof, and the known Windows flake is a separate matter. So: run it at least five times, write the golden from what you observe, and **report how many runs you observed and whether the order held.** If the order does vary, restructure so it cannot — have the client collect its two results and print them after `join`, or print from one side only — and say which shape you settled on.

- [ ] **Step 2: Register the fixture and run it to verify it fails**

Add `http_keepalive_run` to `crates/nova-cli/tests/run_tests.rs` in the same shape as `http_offsets_run`. Run it and confirm it fails with `read_request` not found.

- [ ] **Step 3: Implement `read_request` and `write_response`**

Append to `std/http/lib.nova`:

```nova
// Read one request from `conn`, honouring keep-alive: the caller loops, and
// this reads exactly one request's head and body, leaving anything after it
// on the connection.
//
// **This accumulates the head with `Bytes::concat`, which is O(n) per call**,
// so building an n-byte head out of k reads is O(n*k). Bounded by
// `max_head_bytes` at 8 KiB that is not worth a rope or a growable buffer;
// it is noted here so a reader knows it was weighed rather than missed.
//
// **One socket wait per task.** Staging two I/O waits in one poll ends the
// process (`stage_park` in `crates/nova-runtime/src/task.rs`), so a task
// parked in `accept` cannot also be here. A server needs one task accepting
// and one per connection -- see `std/net`'s `accept` doc comment.
pub async fn read_request(conn: TcpStream, limits: Limits) -> Result<Request, HttpError> {
    let mut buf = bytes_from_string("")
    let mut head: Option<Request> = None
    while head.is_none() {
        let chunk = match conn.read(4096).await {
            Ok(b) => b
            Err(e) => return Err(HttpError { kind: Transport, message: e.message })
        }
        if chunk.len() == 0 {
            return Err(HttpError { kind: Transport, message: "connection closed mid-head" })
        }
        buf = buf.concat(chunk)
        // One arm, not two: `parse_request_head` already returns
        // `Option<Request>`, so the loop assigns it straight through. It is
        // also the form the language permits -- `Ok(Some(r))` is
        // `E0900: nested patterns inside variants are not supported yet`.
        match parse_request_head(buf, limits) {
            Ok(maybe) => head = maybe
            Err(e) => return Err(e)
        }
    }
    let req = match head {
        Some(r) => r
        None => return Err(HttpError { kind: BadRequestLine, message: "no head after parsing" })
    }

    // The head's own length, so the body can be split off what was already
    // read. `parse_offsets` is cheap and the buffer is unchanged, so re-reading
    // `body_start` costs less than threading it out of `parse_request_head`
    // and widening that function's return type for one caller.
    let t = parse_offsets(buf)
    let body_start = t[t.len() - 1]

    let want = content_length_of(req)
    if want > limits.max_body_bytes {
        return Err(HttpError { kind: BodyTooLarge, message: "Content-Length over max_body_bytes" })
    }
    let mut body = buf.slice(body_start, buf.len())
    while body.len() < want {
        let chunk = match conn.read(4096).await {
            Ok(b) => b
            Err(e) => return Err(HttpError { kind: Transport, message: e.message })
        }
        if chunk.len() == 0 {
            return Err(HttpError { kind: Transport, message: "connection closed mid-body" })
        }
        body = body.concat(chunk)
        if body.len() > limits.max_body_bytes {
            return Err(HttpError { kind: BodyTooLarge, message: "body over max_body_bytes" })
        }
    }

    Ok(Request { method: req.method, path: req.path, headers: req.headers, body: body.slice(0, want) })
}

// `Content-Length`, or `0` when it is absent or not a number.
//
// **`Content-Length` is the only body framing v1 understands.** A request
// without one is treated as bodiless; chunked transfer-encoding is out of
// scope and is not detected, so a chunked request's body is left on the
// connection and will be misread as the next request. That is a known
// limitation of v1 rather than a defect to work around here -- it is recorded
// in the design spec's section 10 and in this module's header.
fn content_length_of(req: Request) -> Int {
    match req.headers.get("content-length") {
        Some(v) => digits_to_int(v.trim())
        None => 0
    }
}

// A decimal string as an `Int`, or `0` on anything unexpected. Nova has no
// `String`-to-number conversion, so this walks the characters.
fn digits_to_int(s: String) -> Int {
    let cs = s.chars()
    if cs.len() == 0 { return 0 }
    let mut n = 0
    let mut i = 0
    while i < cs.len() {
        let d = char_digit(cs[i])
        if d < 0 { return 0 }
        n = n * 10 + d
        i = i + 1
    }
    n
}

fn char_digit(c: Char) -> Int {
    if c == '0' { return 0 }
    if c == '1' { return 1 }
    if c == '2' { return 2 }
    if c == '3' { return 3 }
    if c == '4' { return 4 }
    if c == '5' { return 5 }
    if c == '6' { return 6 }
    if c == '7' { return 7 }
    if c == '8' { return 8 }
    if c == '9' { return 9 }
    0 - 1
}

// Write `resp` to `conn`, looping until every byte is out.
//
// **`std/net`'s `write` is one non-blocking attempt, not a `write_all`**, and
// it returns the count it managed -- so a caller that ignored the count would
// silently truncate a response under back-pressure. Returns the total written,
// which equals the response's length on success.
pub async fn write_response(conn: TcpStream, resp: Response) -> Result<Int, IoError> {
    let wire = resp.to_bytes()
    let total = wire.len()
    let mut sent = 0
    while sent < total {
        let n = match conn.write(wire.slice(sent, total)).await {
            Ok(n) => n
            Err(e) => return Err(e)
        }
        if n <= 0 {
            return Err(IoError { kind: Other, message: "write made no progress" })
        }
        sent = sent + n
    }
    Ok(total)
}
```

Also add to the module header, beside the existing notes, one paragraph recording two limitations together, because they share a cause — v1 reads exactly one request's worth of bytes and trusts `Content-Length` to say where it ends:

- `Content-Length` is the only framing v1 understands. Chunked transfer-encoding is neither supported nor detected, so a chunked request's body is left on the connection and will be misread as the next request.
- Bytes read past the current request's body are discarded, so a pipelined request is lost and its connection deadlocks. See the requirement at `read_request` above for the exact wording this needs.

`Option::is_none` and `Option::is_some` are used above, and both are measured present on this tree (`std/core/lib.nova:12` and `:16`, where `is_none` is `!self.is_some()`). `Bytes::eq` likewise exists, through `impl Eq for Bytes` at `std/bytes/lib.nova:115`, and is what Task 4's fixture compares wire bytes with.

- [ ] **Step 4: Run the fixture until it is stably green**

```bash
cargo build --locked --workspace && cargo test --locked --package nova-cli http_keepalive_run --no-fail-fast
```

Run it **at least five times** and report how many passed. This fixture uses real sockets and two tasks, so it is exactly the shape the known Windows flake lands on: if a run fails, re-run, say so, attribute no cause, and fix nothing. A failure that reproduces on every run is not that flake and must be diagnosed.

- [ ] **Step 5: Run the full suite**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum every `test result:` line. Expected: Task 4's total plus one fixture, 0 failed, 8 ignored.

- [ ] **Step 6: Lint, format, byte-scan, and commit**

```bash
cargo clippy --locked --workspace --all-targets -- -D warnings && cargo fmt --all -- --check
```

Byte-scan every file written. Then commit with subject `feat(std/http): read a request from a connection, keep-alive included`. The body records the one-socket-wait-per-task constraint and why the server needs two tasks, that `write_response` loops because `std/net`'s `write` is a single attempt, the O(n) `concat` accumulation and why it was accepted, that `Content-Length` is the only framing v1 understands, and how many times the keep-alive fixture was run. It ends exactly with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Task 6: The records

**Files:**
- Create: `docs/adr/0019-offset-table-intrinsic-boundary.md`
- Modify: `nova-spec/20-STDLIB.md` (section 6, a dated amendment above the code block)
- Modify: `nova-spec/00-MASTER-SPEC.md` (section 3's Phase 2 list at `:240`; the dependency table at `:416`)
- Modify: `CHANGELOG.md` (under `[Unreleased]`)

**Interfaces:**
- Consumes: everything Tasks 1 through 5 shipped. Nothing consumes this task.

- [ ] **Step 1: Write ADR 0019**

Create `docs/adr/0019-offset-table-intrinsic-boundary.md`, following the structure the sibling ADRs use (`## Status`, `## Context`, `## Decision`, `## Consequences`, `## References`). Read `docs/adr/0018-std-json-scope-and-build-order.md` first for the house style: it argues from measurements and names the alternatives it rejected.

It must cover:

- **What the offset table is**, stated exactly as the encoding, and that a non-zero status yields a length-1 array so a caller who skips the check indexes out of bounds. Say that this is deliberate.
- **Why one intrinsic rather than a handle table.** The `File` pattern — a record over a `static` map, reached through the "call, then take" idiom — needs roughly seven intrinsics and two FFI crossings per header, and Nova has no destructors, so a forgotten release leaks. A file handle leaking is a bug; a per-request table entry leaking is unbounded growth under exactly the load the Phase 2 gate measures.
- **Why hyper does not drive the server**, with the three blockers as measured: the executor cannot be re-entered (`IN_BLOCK_ON`, and `run_aborts_when_an_async_fn_calls_block_on` pins it); there are no wakers (`task.rs` names a deadline and another task's completion as the executor's only two wake sources); and ADR 0009 makes single-threading a correctness requirement, so hyper cannot have its own thread. Note that what survives is the master spec's own distinction — hyper's *internals*, which is `httparse`.
- **That this is the first intrinsic in this project to return a structured table** rather than a scalar or a single value, and what a future one should copy: offsets into the caller's buffer rather than copies; no Rust-side state; a status word first; and a length-1 array on any non-zero status. State the property, and let the reader check the roster themselves rather than asserting a closed world over the runtime.
- **The cost, and that it is a risk rather than a settled question.** Eager header materialisation is roughly 20 GC allocations per request at about 900 ns each — about 18 microseconds, about 18% of one core at 10k req/sec, before parsing, I/O or the handler. The escape hatch does not disturb the intrinsic: Nova keeps the offset table and materialises a header only when looked up, and only `Request`'s internals move. **No claim is made that the gate is reached.**
- **That `Map<String, String>` inherits the per-process seeded string hash**, so a map keyed on attacker-supplied header names resists a precomputed collision set. Header parsing is the canonical HashDoS vector and the mitigation was already in place. Do not re-derive it; cite ADR 0005 and its amendments.
- **The three rulings** this plan made (the `Limits` split, `Unknown(String)`, and the scanned allocation), each with its reason.

- [ ] **Step 2: Amend `nova-spec/20-STDLIB.md` section 6**

Insert a dated amendment immediately below the `## 6. std/http (server + client)` heading and above the code block, in the house format that file already uses (`**AMENDED 2026-09-01 (branch \`std-http-parsing\`):** ...`). **Amend; do not rewrite the body.** The body stays as the aspiration it always was.

It must say: the code block below is written in a Nova that does not exist — `[u8]` is `E0001` (the byte type is `Bytes`) and `pub type Handler = async fn(Request) -> Response` is `P0001`. What v1 ships is the server half without a router: `Method`, `Request`, `Response`, `Limits`, `HttpError`, `read_request`, `write_response`, and `parse_offsets`. The client (`get`, `post`, `Response::json`), the router, HTTPS, HTTP/2, chunked transfer-encoding and pipelining are not in v1. Keep-alive is. Header names are lower-cased on insert and the original casing is not preserved. `Content-Length` is the only body framing. `Method` gains an `Unknown(String)` arm the section does not have, and it is named `Unknown` rather than `Other` because `std/io` already exports an `Other` variant.

It must also record the fact the section's own design assumed away: **function types and closures both compile and run on this tree, capture included.** That is what makes a future router possible without a compiler change, provided handlers are synchronous. Do not overclaim it — the async handler type is still `P0001`.

- [ ] **Step 3: Amend `nova-spec/00-MASTER-SPEC.md`**

Two edits, each with its reason stated inline rather than left implicit.

Section 3's Phase 2 list at `:240` reads:

```text
10. `std/http` (server first, then client; use hyper internals at runtime layer)
```

Narrow "use hyper internals at runtime layer" to the **parsing** internals, and give the reason: hyper's runtime is unavailable here for the three measured blockers in ADR 0019, and `httparse` is what its HTTP/1 parsing actually is.

The dependency table at `:416` lists:

```toml
hyper = { version = "1", features = ["full"] }
```

Record what shipped instead: `httparse = "1.10"` in `crates/nova-runtime`, with no dependencies of its own, and a pointer to ADR 0019 for why. Whether the `hyper` line stays as an aspiration for the client half is your call — if you keep it, say what it is still for; if you replace it, say what was replaced and why.

**Also record, without fixing, two pieces of example drift** this increment noticed. Both were measured on this tree rather than recalled, and both are stated over the whole population rather than the one example that first drew attention.

First, the numbering: `nova-spec/00-MASTER-SPEC.md` section 3's tree names `03-http-server`, and `nova-spec/60-EXAMPLES.md` section 3 names it too, while `examples/` holds `03-producer-consumer`. **The drift is in two spec files, not one** -- a note added to only the master spec would leave the other saying the same wrong thing.

Second, the READMEs: `nova-spec/60-EXAMPLES.md` section 9 gives a per-example README template, and **no example in `examples/` has one** -- not `01-hello-world`, not `02-fibonacci`, not `03-producer-consumer`. An earlier draft of this plan named only the third, which was true of it and misleading about the other two.

Neither is this increment's to fix; both are its to record. Put the note where a reader of the tree will find it.

- [ ] **Step 4: Add the CHANGELOG entry**

Under `[Unreleased]`, in the `### Added` section, following the style of the entries already there — they are dense, they name the measured numbers, and they say what was deliberately not done.

It must cover: `std/http` as a `STD_MODULES` entry (13 to 14); the one new intrinsic `http_parse_request` (`Builtin::STD_ONLY` 70 to 71) and what it returns; `httparse` as the one new dependency, with no dependencies of its own; that hyper does not drive the server and the three reasons; the surface that shipped and the surface that did not; the lower-casing and `Content-Length`-only limitations; the `Unknown(String)` naming and its reason; and the eager-materialisation cost as an open risk with its escape hatch named. **No claim that the Phase 2 gate is reached** — this makes it measurable, which it was not.

- [ ] **Step 5: Sweep the records for stale claims**

grep is line-oriented, so a miss is **not** evidence of absence. Sweep with whitespace-tolerant patterns that also normalise `//`, `///` and `>` gutters — a plain line-oriented grep returned zero hits on a phrase that exists, twice, on an earlier branch of this project.

Things to sweep for, across `nova-spec/`, `docs/`, `std/`, `crates/` and `CHANGELOG.md`:

- Any sentence saying `std/http` is absent, missing, or not yet built. Two Phase 2 module groups were missing; one still is (`std/crypto`). A sentence that says "the two missing module groups" is now wrong, and correcting the number is usually the wrong fix — prefer naming `std/crypto`.
- Any sentence claiming Phase 2's gate cannot be measured because `std/http` does not exist.
- Any count of `STD_MODULES` entries, `STD_ONLY` builtins, or workspace dependencies stated as a bare number.
- Any roster of std modules that would now be incomplete.

Report what you found and what you changed. **Fix every artifact the sweep names, not only the one you happen to be editing** — the recurring failure on this project is fixing the artifact in hand and leaving the tracked ones that say the same thing.

- [ ] **Step 6: Byte-scan and commit**

Byte-scan every file written, per the global constraints, including the planted-positive assertion for the backslash-u pattern. Add one check to that scan: none of this branch's own commit SHAs may appear in any tracked file. Build the roster with `git log --format=%h main..HEAD` and assert each is absent — do not type a SHA into the scan by hand, since that would put it in a tracked file if the scan itself is ever committed.

```bash
git add docs/adr nova-spec CHANGELOG.md
git commit -F <path-to-message-file>
```

Subject: `docs(std/http): record the offset-table boundary and narrow the hyper strategy`. The body says what ADR 0019 decides, which two spec sections were amended and how, what the sweep found, and that the example-numbering drift was recorded rather than fixed. It ends exactly with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Self-Review

Run against the spec after the plan was complete.

**1. Spec coverage.** Section 1 (scope and the three exclusions): Task 3's module header and Task 6's Step 2. Section 2 (hyper unavailable, three blockers): Task 6's ADR and Step 3. Section 3 (accept loop, verified language facts): Task 5, and the fixture in Task 5 Step 1. Section 4 and 4.1 (one intrinsic, the exact encoding): Task 1, pinned from both sides by Task 1 Step 3 and Task 3 Step 1. Section 5 (the Nova API): Tasks 3, 4 and 5, with the `Other`-to-`Unknown` deviation recorded. Section 6 (limits, no panic): Task 1's constants and guard, Task 3's `Limits` and fixture. Section 7 (cost): Task 6's ADR. Section 8 (tests and mutations): every test named there has a task, and each of the four mutations is run — 2 and 3 in Task 1, 4 in Task 3, 1 in Task 4. Section 9 (records): Task 6. Section 10 (limitations): Task 3's header, Task 5's `content_length_of`, Task 6's amendment. Section 11 (success criteria): criterion 1's `--all-targets` requirement is in the plan header and Task 2; criterion 2 is Task 4 Step 6; criterion 3 is Task 5; criterion 4 is Task 3 Step 7; criterion 5 is every task's suite step; criterion 6 is Task 1 Step 2.

**No gaps found**, with one thing worth flagging to the executor rather than hiding: spec section 5's `read_request` returns `Result<Request, HttpError>` and section 8 asks for a keep-alive test, but the spec never says what closes a connection. This plan's Task 5 has `read_request` return a `Transport` error on a zero-length read and leaves the closing decision to the caller's loop, which is what the spec's own usage sketch does. If a reviewer wants an explicit `Connection: close` check, that is a change to the spec, not a defect in the plan.

**2. Placeholder scan.** Two goldens are deliberately written as "measure, then fill in" rather than as invented numbers — `tests/runtime/http_malformed.stdout` and the two byte counts in `tests/runtime/http_serialise.stdout` — and both say so explicitly with the reason. Those are instructions to measure, not placeholders: a guessed golden that the implementation is then bent to match is worse than no golden. Every other step carries its actual content.

**3. Type consistency.** `parse_offsets(buf: Bytes) -> [Int]` is named identically in Tasks 3, 4 and 5. `parse_request_head(buf, limits) -> Result<Option<Request>, HttpError>` is consistent between its definition in Task 3 and its two call sites. `Response::to_bytes(self) -> Bytes` is defined in Task 4 and called in Task 5. `write_response` returns `Result<Int, IoError>` — **spec section 5 writes it as `Result<(), IoError>`**; this plan returns the byte count instead, because `std/net`'s `write` returns a count and discarding it at this layer would hide a truncated write. That deviation is deliberate and is called out here so a reviewer sees it rather than finding it.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-01-std-http-request-parsing.md`. Two execution options:

**1. Subagent-Driven (recommended)** — a fresh subagent per task, with a spec-compliance and code-quality review between tasks, and a broad whole-branch review at the end.

**2. Inline Execution** — execute the tasks in this session using `superpowers:executing-plans`, batching with checkpoints for review.

Which approach?
