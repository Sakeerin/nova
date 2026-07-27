# Phase 2.2b — `std/strings` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship codepoint-level string operations as a third embedded std module, built on five primitive intrinsics, and fix `Debug for String` so it produces valid Nova literals.

**Architecture:** Five new `Builtin::STD_ONLY` intrinsics expose what Nova cannot do to a string (decode, encode, count codepoints, case-map). Every algorithm — slicing, searching, splitting, trimming — is written in Nova in `std/strings/lib.nova` as one inherent `impl String` block over `[Char]`, with private non-`pub` top-level helpers. Both codegen backends need no changes, because `RtFunc::ALL` already drives their declarations.

**Tech Stack:** Rust (nova-resolver, nova-typeck, nova-mir, nova-runtime), Nova (std/strings, std/core), `cargo test` + committed `tests/runtime` stdout fixtures.

**Spec:** `docs/superpowers/specs/2026-07-27-phase-2-2b-strings-design.md`. Read §4.2 before writing any method — it pins eleven cases that have a defensible opposite answer.

## Global Constraints

- Run every `cargo` command from `D:\Projects\nona\nova`.
- Must end green on all three: `cargo test --workspace --no-fail-fast`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`. **`--no-fail-fast` is mandatory** — without it cargo abandons later targets and under-reports.
- `cargo build -p nova-cli` after **any** `std/*.nova` edit before probing by hand, or a stale binary gives misleading errors. Every program in the test suite compiles all std modules, so many unrelated failures at once means your Nova source is wrong.
- **After editing `crates/nova-runtime/`, run `cargo build --workspace` BEFORE `cargo test`.** `cargo test` does **not** regenerate `nova-runtime`'s staticlib (`target/debug/nova_runtime.lib`), which `nova build` links standalone executables against — measured: touching `nova-runtime/src/lib.rs` and running `cargo test` leaves the staticlib's mtime unchanged. So a newly added `nova_rt_*` symbol is missing from the stale `.lib`, and roughly 25 `*_build_standalone` tests fail with an MSVC `unresolved external symbol` error that looks like a codegen bug and is not one. This is pre-existing repo behaviour, not something any task here introduces; `crates/nova-driver/src/link.rs:104` already knows about the general case and suggests `cargo build -p nova-runtime`. Tasks 3, 4 and 8 add runtime symbols and will each hit this.
- The two existing gates must keep passing byte-identically: `tests/runtime/collections.nova`, `tests/runtime/std_core.nova`.
- `tests/runtime/*.stdout` fixtures are **CRLF** in the checkout while the compiler emits **LF**. The harness normalises with `.replace("\r\n", "\n")`; compare by hand with `tr -d '\r'`, never a raw diff.
- No `unwrap()`/`expect()` in Rust library paths reachable from user input; prefer `.get(..)`. Tests may use them.
- **Nova language limits, all verified against this compiler:** no tuples; no references; `for` only over integer ranges; `///` doc comments do **not** parse in Nova source — use `//`; `>>` is arithmetic with no `>>>`; no `mut` on record fields; no field privacy. **`String + String` is `E0013`** — there is no `+` for strings, so build every result through `str_from_chars` over a `[Char]` or through `"${a}${b}"` interpolation.
- **`break`/`return` followed by a newline and then an expression parses that expression as the value** (an ASI-style pitfall). Always place `break` immediately before a `}`.
- Conventional commits, each ending with exactly: `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`
- **Never `git push`.** The user pushes explicitly.
- If a pre-existing test changes behaviour, investigate — do not edit it to match without understanding why.

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `std/strings/lib.nova` | **New.** The whole public surface as one inherent `impl String` block, plus private non-`pub` top-level helpers. | S1–S8 |
| `crates/nova-resolver/src/lib.rs` | `Builtin` variants + `name()` + `STD_ONLY` membership; `STD_MODULES` grows to 3 entries. | S1–S4, S8 |
| `crates/nova-typeck/src/check.rs` | `builtin_signature` arms; the diagnostic `hint` match. | S2–S4, S8 |
| `crates/nova-mir/src/lib.rs` | `RtFunc` variants + `symbol()` + `signature()`. | S2–S4, S8 |
| `crates/nova-mir/src/lower.rs` | The exhaustive `hir::Callee::Builtin(b)` match. | S2–S4, S8 |
| `crates/nova-runtime/src/lib.rs` | The five `extern "C"` functions + `symbols()` registration. | S2–S4, S8 |
| `std/core/lib.nova` | `Debug for String` fix; correct the stale comment at :168. | S9 |
| `tests/runtime/strings.{nova,stdout}` | **New.** The phase gate. | S10 |
| `crates/nova-cli/tests/run_tests.rs` | Three gate tests (run / build / GC stress). | S10 |
| `CHANGELOG.md` | New surface + the 18 shadowed method names. | S10 |

### Adding one builtin touches six places (verified, all exhaustive matches)

Three of these will not compile until updated, which is deliberate:

1. `nova-resolver/src/lib.rs` — `Builtin` variant, its `name()` arm, and `STD_ONLY` (**bump the array's length annotation**).
2. `nova-typeck/src/check.rs` — `builtin_signature` arm.
3. `nova-typeck/src/check.rs` — the `hint` match (`Builtin::StrCmp | StrHash | CharToInt => ""`). **Exhaustive.**
4. `nova-mir/src/lower.rs` — the `hir::Callee::Builtin(b)` match. **Exhaustive by design** ("`None` is not 'unhandled' but 'handled without one'").
5. `nova-mir/src/lib.rs` — `RtFunc` variant, `symbol()`, `signature()`. **Exhaustive.**
6. `nova-runtime/src/lib.rs` — the `extern "C"` fn plus its entry in `symbols()` (needed for the JIT; `nova run` fails to resolve the symbol without it).

Neither codegen backend changes: `RtFunc::ALL` is their single source of truth, and `every_rt_func_is_declared_with_its_real_signature` fails if a variant is left unwired.

---

### Task 1 (S1): Third std module scaffold

Creates `std/strings` as `STD_MODULES[2]` with the one method that needs **no** new intrinsic, so the module-wiring change is gated independently of any ABI work.

**Files:**
- Create: `std/strings/lib.nova`
- Modify: `crates/nova-resolver/src/lib.rs:509-515` (`STD_MODULES`)
- Test: `crates/nova-cli/tests/run_tests.rs`, `crates/nova-resolver/src/lib.rs` (tests module)

**Interfaces:**
- Consumes: nothing.
- Produces: `std/strings/lib.nova` exists and is `STD_MODULES[2]`; `String::is_empty(self) -> Bool` is callable from any user program. Later tasks add methods to the **same single** `impl String` block in this file.

- [ ] **Step 1: Write the failing test**

In `crates/nova-cli/tests/run_tests.rs`:

```rust
/// `std/strings` is the third embedded std module (Phase 2.2b). `is_empty` is
/// the one method in it that needs no new intrinsic, so this pins that the
/// module is loaded and its inherent `impl String` resolves — independently of
/// the five intrinsics the rest of the surface needs.
#[test]
fn std_strings_module_is_loaded() {
    let src = "fn main() { println(\"${\"\".is_empty()} ${\"x\".is_empty()}\") }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-scaffold");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("true false\n");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests std_strings_module_is_loaded -- --nocapture`
Expected: FAIL — `E0015`/`E0014`-family error, because no `is_empty` exists on `String`.

- [ ] **Step 3: Create `std/strings/lib.nova`**

```nova
// Nova standard library — strings.
//
// Compiled as an implicit module and glob-imported into every user module, so
// these names need no `import` (see docs/adr/0004-stdlib-compile-model.md).
//
// EVERY index and length here is in CODEPOINTS (Unicode scalar values), never
// bytes. `NovaStr` stores UTF-8 and its own `len` field is a byte count, but
// no part of this module's surface exposes that.
//
// PERFORMANCE CONTRACT, worth reading before using this module in a loop:
// every operation that inspects the string decodes the whole thing into a
// `[Char]` first, so `char_at` is O(n), and calling it for each index is
// QUADRATIC. Call `chars()` once and index the resulting array instead. This
// is the same class of rule as the `hash & (cap - 1)` note beside
// `pub trait Hash` in std/core — an invariant a caller has to know.
//
// This is the first inherent `impl String` in the language. Keep it as ONE
// block: whether two inherent impls on the same primitive in different std
// modules is rejected (`E0074` is specified for *trait* impls) or silently
// accepted with one shadowing the other is untested. An inherent method also
// wins by priority over a same-named trait method, so no method here may be
// named `fmt`, `dbg`, `eq`, `ne`, `cmp`, `clone`, `default` or `hash` — those
// are std/core's trait methods on `String` and would be silently shadowed.

impl String {
    // Not `self.len() == 0`: `len()` decodes the whole string, while `==` on
    // `String` compares byte lengths first in the runtime.
    pub fn is_empty(self) -> Bool { self == "" }
}
```

- [ ] **Step 4: Register it as the third std module**

In `crates/nova-resolver/src/lib.rs`, change the length annotation from `2` to `3` and add the entry:

```rust
pub const STD_MODULES: [(&str, &str); 3] = [
    ("$std.core", include_str!("../../../std/core/lib.nova")),
    (
        "$std.collections",
        include_str!("../../../std/collections/lib.nova"),
    ),
    (
        "$std.strings",
        include_str!("../../../std/strings/lib.nova"),
    ),
];
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo build -p nova-cli` then `cargo test -p nova-cli --test run_tests std_strings_module_is_loaded`
Expected: PASS, printing `true false`.

- [ ] **Step 6: Verify nothing else regressed**

Run: `cargo test --workspace --no-fail-fast`
Expected: all green. `std_only_builtins_are_visible_inside_std_modules` loops `1..=STD_MODULES.len()`, so it now covers three modules automatically — if it fails, the new module did not get the std-only builtins and that is a real finding, not a test to edit.

- [ ] **Step 7: Commit**

```bash
git add std/strings/lib.nova crates/nova-resolver/src/lib.rs crates/nova-cli/tests/run_tests.rs
git commit -m "feat(std): add std/strings as the third embedded std module"
```

---

### Task 2 (S2): `str_len_chars` + `String::len`

The simplest possible new intrinsic, done end-to-end through all six touchpoints — so a mistake in the wiring pattern surfaces here, isolated from S3's array-layout risk.

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs` (`Builtin`, `name()`, `STD_ONLY`), `crates/nova-typeck/src/check.rs` (`builtin_signature`, `hint` match), `crates/nova-mir/src/lib.rs` (`RtFunc`), `crates/nova-mir/src/lower.rs` (builtin match), `crates/nova-runtime/src/lib.rs` (fn + `symbols()`), `std/strings/lib.nova`

**Interfaces:**
- Consumes: `std/strings/lib.nova` from S1.
- Produces: `str_len_chars(String) -> Int` (std-only builtin), `RtFunc::StrLenChars`, `nova_rt_str_len_chars`, and `String::len(self) -> Int`.

- [ ] **Step 1: Write the failing test**

In `crates/nova-cli/tests/run_tests.rs`:

```rust
/// `String::len` counts CODEPOINTS, not bytes — the whole point of Phase 2.2b.
/// `café` is 5 UTF-8 bytes but 4 codepoints; each CJK character here is 3
/// bytes. A byte-based implementation prints `5` and `9`.
#[test]
fn string_len_counts_codepoints_not_bytes() {
    let src = "fn main() { println(\"${\"café\".len()} ${\"日本語\".len()} ${\"\".len()}\") }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-len");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("4 3 0\n");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests string_len_counts_codepoints_not_bytes`
Expected: FAIL — no `len` on `String`.

- [ ] **Step 3: Add the runtime function**

In `crates/nova-runtime/src/lib.rs`, beside `nova_rt_str_hash`:

```rust
/// Number of Unicode scalar values in `s`.
///
/// Separate from [`nova_rt_str_chars`] so that asking a string's length does
/// not allocate an array of its characters.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_len_chars(s: *const NovaStr) -> i64 {
    as_str(s).chars().count() as i64
}
```

And in `symbols()`:

```rust
        ("nova_rt_str_len_chars", nova_rt_str_len_chars as *const u8),
```

- [ ] **Step 4: Add a runtime unit test**

In `crates/nova-runtime/src/lib.rs`'s `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn str_len_chars_counts_scalars_not_bytes() {
        unsafe {
            assert_eq!(nova_rt_str_len_chars(make_str("café")), 4);
            assert_eq!(nova_rt_str_len_chars(make_str("日本語")), 3);
            assert_eq!(nova_rt_str_len_chars(make_str("")), 0);
            // A 4-byte scalar outside the BMP is still one character.
            assert_eq!(nova_rt_str_len_chars(make_str("🦀")), 1);
        }
    }
```

- [ ] **Step 5: Wire the builtin through the four compiler touchpoints**

`crates/nova-resolver/src/lib.rs` — add the variant after `CharToInt`:

```rust
    /// `str_len_chars(s: String) -> Int` — the number of Unicode scalar
    /// values in `s`. Backs `std/strings`' `String::len`. Nova cannot walk a
    /// string (`String` has no length, indexing or iteration) and cannot
    /// reach the runtime through an `extern` either (`String` is not
    /// FFI-safe). Separate from `str_chars` so a length query allocates
    /// nothing. Std-only, so it is not a reserved word in user code.
    StrLenChars,
```

its `name()` arm:

```rust
            Builtin::StrLenChars => "str_len_chars",
```

and `STD_ONLY` (**note the length annotation**):

```rust
    pub const STD_ONLY: [Builtin; 4] = [
        Builtin::StrCmp,
        Builtin::StrHash,
        Builtin::CharToInt,
        Builtin::StrLenChars,
    ];
```

`crates/nova-typeck/src/check.rs` — `builtin_signature`:

```rust
        Builtin::StrLenChars => (vec![Ty::String], Ty::Int),
```

and add `Builtin::StrLenChars` to the `=> ""` arm of the `hint` match.

`crates/nova-mir/src/lib.rs` — the `RtFunc` variant, `symbol()` and `signature()`:

```rust
    /// `(str) -> i64` — count of Unicode scalar values.
    StrLenChars,
```
```rust
            RtFunc::StrLenChars => "nova_rt_str_len_chars",
```
```rust
            RtFunc::StrLenChars => (vec![MirTy::Ptr], MirTy::I64),
```

`crates/nova-mir/src/lower.rs` — the builtin match:

```rust
                    Builtin::StrLenChars => Some(RtFunc::StrLenChars),
```

- [ ] **Step 6: Add the Nova method**

In `std/strings/lib.nova`, inside the existing `impl String` block:

```nova
    // Codepoints, not bytes. O(n): the runtime walks the UTF-8.
    pub fn len(self) -> Int { str_len_chars(self) }
```

- [ ] **Step 7: Confirm array `.len()` still resolves**

`arr.len()` is special-cased in typeck for `Ty::Array`. Adding a `len` method on `String` must not disturb it.

Run: `cargo test --workspace --no-fail-fast`
Expected: all green, including every existing array and `Vec`/`Map`/`Set` `len` test. If an array `len` test fails, the special case and method resolution are interacting — that is a compiler finding to report, not something to work around.

- [ ] **Step 8: Run the new tests**

Run: `cargo build -p nova-cli` then `cargo test -p nova-cli --test run_tests string_len_counts_codepoints_not_bytes` and `cargo test -p nova-runtime str_len_chars`
Expected: both PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/ std/strings/lib.nova
git commit -m "feat(std): add str_len_chars and String::len (codepoints, not bytes)"
```

---

### Task 3 (S3): `str_chars` + `String::chars` + `String::char_at`

**The highest-risk task in the plan.** `str_chars` is the first intrinsic that constructs a Nova array in the runtime, so it must reproduce codegen's layout exactly. A mistake is a silent miscompile, not a crash — which is why Step 1's test reads the array back from Nova rather than trusting Rust-side inspection.

**Files:**
- Modify: the same six touchpoints as S2, plus `std/strings/lib.nova`

**Interfaces:**
- Consumes: S2's wiring pattern.
- Produces: `str_chars(String) -> [Char]`, `RtFunc::StrChars`, `nova_rt_str_chars`, `String::chars(self) -> [Char]`, `String::char_at(self, i: Int) -> Option<Char>`.

- [ ] **Step 1: Write the failing test**

```rust
/// `str_chars` is the first intrinsic to build a Nova array in the runtime, so
/// it must reproduce codegen's `{ len, elems at 8 + 8*i }` layout exactly. A
/// wrong offset or a wrong length header is a SILENT MISCOMPILE, not a crash —
/// so this reads `.len()` back and indexes elements from Nova, which is the
/// only thing that actually exercises the layout the compiler assumes.
#[test]
fn str_chars_array_matches_codegen_layout() {
    let src = "fn main() {\n\
               let cs = \"a→🦀\".chars()\n\
               println(\"${cs.len()} ${cs[0]} ${cs[1]} ${cs[2]}\")\n\
               let e = \"\".chars()\n\
               println(\"${e.len()}\")\n\
               println(\"${\"héllo\".char_at(1).unwrap_or('?')} \
               ${\"héllo\".char_at(9).unwrap_or('?')} \
               ${\"héllo\".char_at(0 - 1).unwrap_or('?')}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-chars");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("3 a → 🦀\n0\né ? ?\n");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests str_chars_array_matches_codegen_layout`
Expected: FAIL — no `chars` on `String`.

- [ ] **Step 3: Add the runtime function**

```rust
/// Decompose `s` into a Nova `[Char]`.
///
/// The result must match **exactly** what codegen emits for an array: one
/// block holding `{ len: i64, elem0, elem1, … }`, element `i` at byte offset
/// `8 + 8*i`, allocated *scanned* the way [`nova_rt_alloc`] allocates (it
/// takes no scan parameter and always scans). A `Char` element is its `i64`
/// Unicode scalar value, because `Ty::Char` and `Ty::Int` are both
/// `MirTy::I64`.
///
/// Scanning an array of scalars can retain garbage that happens to look like
/// a pointer. That is the conservative collector's existing behaviour for any
/// `[Int]`, not something new here.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_chars(s: *const NovaStr) -> *mut u8 {
    let chars: Vec<char> = as_str(s).chars().collect();
    let n = chars.len();
    // `8` for the length header plus `8` per element — the same size
    // arithmetic `nova_rt_alloc` is asked for when codegen builds an array.
    // A char count cannot overflow this on a 64-bit target, and `gc::alloc`
    // rejects an undescribable size regardless.
    let block = gc::alloc(8 + 8 * n, true);
    let words = block as *mut i64;
    *words = n as i64;
    for (i, c) in chars.iter().enumerate() {
        *words.add(1 + i) = *c as i64;
    }
    block
}
```

And in `symbols()`: `("nova_rt_str_chars", nova_rt_str_chars as *const u8),`

- [ ] **Step 4: Add a runtime unit test for the layout**

```rust
    #[test]
    fn str_chars_writes_the_array_layout_codegen_expects() {
        unsafe {
            let block = nova_rt_str_chars(make_str("a→🦀"));
            let words = block as *const i64;
            // Length header first, then one i64 scalar per element.
            assert_eq!(*words, 3);
            assert_eq!(*words.add(1), 'a' as i64);
            assert_eq!(*words.add(2), '→' as i64);
            assert_eq!(*words.add(3), '🦀' as i64);
            // An empty string still yields a well-formed zero-length array.
            assert_eq!(*(nova_rt_str_chars(make_str("")) as *const i64), 0);
        }
    }
```

- [ ] **Step 5: Wire the builtin**

Same six touchpoints as S2. The pieces that differ:

```rust
    /// `str_chars(s: String) -> [Char]` — the string's Unicode scalar values
    /// in order. Backs `std/strings`' `String::chars`, and every operation in
    /// that module that inspects a string. Also backs `std/core`'s
    /// `impl Debug for String`, which needs to escape the contents. Std-only.
    StrChars,
```
```rust
            Builtin::StrChars => "str_chars",
```
```rust
        Builtin::StrChars => (vec![Ty::String], Ty::Array(Box::new(Ty::Char))),
```
```rust
    /// `(str) -> ptr` — a Nova `[Char]`.
    StrChars,
```
```rust
            RtFunc::StrChars => "nova_rt_str_chars",
```
```rust
            RtFunc::StrChars => (vec![MirTy::Ptr], MirTy::Ptr),
```
```rust
                    Builtin::StrChars => Some(RtFunc::StrChars),
```

Add `Builtin::StrChars` to `STD_ONLY` (now `[Builtin; 5]`) and to the `hint` match's `=> ""` arm.

- [ ] **Step 6: Add the Nova methods**

```nova
    // The string's codepoints. Prefer this over repeated `char_at` — see the
    // performance contract in the module header.
    pub fn chars(self) -> [Char] { str_chars(self) }

    // The codepoint at `i`, or `None` when `i` is outside `0..len()`. A
    // negative index is `None` rather than a panic, matching `Vec::get`.
    pub fn char_at(self, i: Int) -> Option<Char> {
        if i < 0 { return None }
        let cs = str_chars(self)
        if i >= cs.len() { None } else { Some(cs[i]) }
    }
```

- [ ] **Step 6b: Make the signature-table test impossible to forget**

`builtin_signatures_are_what_the_std_call_sites_use` (`crates/nova-typeck/src/check.rs:~9226`) is a hand-written list of `assert_eq!`s, one per builtin. Task 2 added `Builtin::StrLenChars` and did **not** add an entry — nothing caught it, and `builtin_signature`'s own doc comment claims that test covers the table. Four more builtins arrive in this task and Tasks 4 and 8, so the list will keep drifting.

Convert it so omission is a **compile error**, which is how this repo already protects its other builtin tables (three separate `match`es are deliberately exhaustive for exactly this reason, and `no_std_only_builtin_is_a_reserved_word` loops `STD_ONLY` so "adding a builtin to it without deciding this question cannot happen"). Rewrite the test body as an exhaustive `match` over `Builtin` that names each variant's expected signature, then assert `builtin_signature(b)` equals it:

```rust
    /// Written as an exhaustive `match` rather than a list of `assert_eq!`s so
    /// that adding a `Builtin` without stating its expected signature does not
    /// compile. The previous hand-written list silently missed `StrLenChars`.
    #[test]
    fn builtin_signatures_are_what_the_std_call_sites_use() {
        // The site each signature has to satisfy, so a mismatch names the
        // caller it would break rather than only the types.
        fn expected(b: Builtin) -> ((Vec<Ty>, Ty), &'static str) {
            match b {
                Builtin::Println | Builtin::Print => {
                    ((vec![Ty::String], Ty::Unit), "`println(s)` / `print(s)`")
                }
                Builtin::Panic => ((vec![Ty::String], Ty::Never), "`panic(msg)` diverges"),
                Builtin::StrCmp => (
                    (vec![Ty::String, Ty::String], Ty::Int),
                    "`str_cmp(self, other)` in `impl Ord for String`",
                ),
                Builtin::StrHash => (
                    (vec![Ty::String], Ty::Int),
                    "`str_hash(self)` in `impl Hash for String`",
                ),
                Builtin::CharToInt => (
                    (vec![Ty::Char], Ty::Int),
                    "`char_to_int(self)` in `impl Hash for Char`",
                ),
                Builtin::StrLenChars => (
                    (vec![Ty::String], Ty::Int),
                    "`str_len_chars(self)` in `String::len`",
                ),
                Builtin::StrChars => (
                    (vec![Ty::String], Ty::Array(Box::new(Ty::Char))),
                    "`str_chars(self)` in `String::chars`",
                ),
            }
        }
        for b in ALL_BUILTINS {
            let (sig, site) = expected(b);
            assert_eq!(builtin_signature(b), sig, "{}: {site}", b.name());
        }
    }
```

This needs a list of every variant to iterate. `Builtin` has no `ALL` constant today — check first whether one exists; if not, add `pub const ALL: [Builtin; N]` to `impl Builtin` in `crates/nova-resolver/src/lib.rs` beside `GLOBAL` and `STD_ONLY`, and use `Builtin::ALL` instead of a test-local `ALL_BUILTINS`. **A length-typed array plus the exhaustive `match` together mean a new variant cannot be added without stating its signature.** Add a short doc comment on `ALL` saying that is its purpose.

Then correct `builtin_signature`'s doc comment if it now overstates or understates what the test covers.

- [ ] **Step 7: Run the tests**

Run: `cargo build -p nova-cli`, then `cargo test -p nova-cli --test run_tests str_chars_array_matches_codegen_layout`, `cargo test -p nova-runtime str_chars`, `cargo test -p nova-typeck builtin_signatures`, then `cargo test --workspace --no-fail-fast`
Expected: all PASS.

- [ ] **Step 8: Prove the layout test actually bites**

Temporarily change `*words = n as i64;` to `*words = (n as i64) + 1;`, run `cargo build -p nova-cli` and the Nova-level test.
Expected: it **FAILS** (a wrong length header changes `cs.len()` and lets `cs[n]` read past the elements). Then restore, rebuild, and confirm it passes again. Report what you saw — a layout test that cannot distinguish a broken layout is worthless.

- [ ] **Step 9: Commit**

```bash
git add crates/ std/strings/lib.nova
git commit -m "feat(std): add str_chars, String::chars and String::char_at"
```

---

### Task 4 (S4): `str_from_chars` + `slice` + `reverse`

Closes the round-trip, and adds the two methods that only need encoding.

**Files:**
- Modify: the six touchpoints, plus `std/strings/lib.nova`

**Interfaces:**
- Consumes: `str_chars` (S3).
- Produces: `str_from_chars([Char]) -> String`, `RtFunc::StrFromChars`, `nova_rt_str_from_chars`; private Nova helper `chars_to_string(cs: [Char], start: Int, end: Int) -> String`; `String::slice(self, start: Int, end: Int) -> String`; `String::reverse(self) -> String`.

- [ ] **Step 1: Write the failing test**

```rust
/// Round-trip and the half-open slice boundary. Per spec §4.2, `slice` is
/// `start` inclusive / `end` exclusive, `start == end` is valid and yields
/// "", and `reverse` reverses codepoints.
#[test]
fn str_from_chars_round_trips_and_slice_is_half_open() {
    let src = "fn main() {\n\
               println(\"${\"a→🦀é\".chars().len()}\")\n\
               println(\"${\"héllo wörld\".slice(0, 5)}|${\"héllo\".slice(0, 0)}|\
               ${\"héllo\".slice(5, 5)}|${\"héllo\".slice(0, 5)}\")\n\
               println(\"${\"a→🦀\".reverse()} ${\"\".reverse()}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-slice");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("4\nhéllo|||héllo\n🦀→a \n");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests str_from_chars_round_trips_and_slice_is_half_open`
Expected: FAIL — no `slice` on `String`.

- [ ] **Step 3: Add the runtime function**

```rust
/// Encode a Nova `[Char]` back into a string.
///
/// A word that is not a valid Unicode scalar value becomes
/// [`char::REPLACEMENT_CHARACTER`] rather than aborting, matching what
/// [`nova_rt_char_to_str`] already does. Nova source cannot produce one —
/// there is no `Int` → `Char` conversion in the language (`let c: Char = 65`
/// is `E0010`, `'a' + 1` is `E0010`, and no such builtin exists) — so this is
/// defensive only.
///
/// # Safety
/// `cs` must point to a Nova array of `Char`: `{ len: i64, elems… }` with
/// element `i` at byte offset `8 + 8*i`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_from_chars(cs: *const u8) -> *mut NovaStr {
    let words = cs as *const i64;
    let n = (*words).max(0) as usize;
    let mut out = String::new();
    for i in 0..n {
        let v = *words.add(1 + i);
        out.push(char::from_u32(v as u32).unwrap_or(char::REPLACEMENT_CHARACTER));
    }
    gc_str(&out)
}
```

And in `symbols()`: `("nova_rt_str_from_chars", nova_rt_str_from_chars as *const u8),`

- [ ] **Step 4: Add a runtime unit test**

```rust
    #[test]
    fn str_from_chars_round_trips_and_substitutes_invalid_scalars() {
        unsafe {
            for s in ["", "ascii", "café", "日本語", "🦀🇹🇭"] {
                let back = nova_rt_str_from_chars(nova_rt_str_chars(make_str(s)));
                assert_eq!(nova_rt_str_eq(back, make_str(s)), 1, "round-trip {s}");
            }
            // A surrogate is not a scalar value; substitute, do not abort.
            let block = gc::alloc(16, true) as *mut i64;
            *block = 1;
            *block.add(1) = 0xD800;
            let s = nova_rt_str_from_chars(block as *const u8);
            assert_eq!(as_str(s), "\u{FFFD}");
        }
    }
```

- [ ] **Step 5: Wire the builtin**

```rust
    /// `str_from_chars(cs: [Char]) -> String` — encode codepoints as UTF-8.
    /// The inverse of [`Builtin::StrChars`], and how every `std/strings`
    /// operation that produces a string produces it: Nova has no `+` for
    /// `String` (`E0013`), so building a result any other way would mean
    /// quadratic interpolation. Std-only.
    StrFromChars,
```
```rust
            Builtin::StrFromChars => "str_from_chars",
```
```rust
        Builtin::StrFromChars => (vec![Ty::Array(Box::new(Ty::Char))], Ty::String),
```
```rust
    /// `(ptr to [Char]) -> str`
    StrFromChars,
```
```rust
            RtFunc::StrFromChars => "nova_rt_str_from_chars",
```
```rust
            RtFunc::StrFromChars => (vec![MirTy::Ptr], MirTy::Ptr),
```
```rust
                    Builtin::StrFromChars => Some(RtFunc::StrFromChars),
```

`STD_ONLY` becomes `[Builtin; 6]`; add to the `hint` match too.

- [ ] **Step 6: Add the private helper and the two methods**

At the **top level** of `std/strings/lib.nova` (not inside the impl). Non-`pub` top-level functions are genuinely module-private — the resolver only glob-imports `pub` names — unlike non-`pub` *methods*, which are not call-gated at all:

```nova
// The codepoints `start..end` of `cs` as a string. `end` is exclusive.
// Callers have already validated the range.
fn chars_to_string(cs: [Char], start: Int, end: Int) -> String {
    let n = end - start
    if n <= 0 { return "" }
    // `[' '; n]` needs a filler; every slot is overwritten immediately below,
    // so the space never survives. A repeat literal is the only way to
    // allocate a runtime-length array.
    let mut out = [' '; n]
    for i in 0..n { out[i] = cs[start + i] }
    str_from_chars(out)
}
```

Inside `impl String`:

```nova
    // Codepoints `start..end`, with `end` EXCLUSIVE (half-open, like every
    // other index range in the language). `start == end` yields "".
    // Panics rather than returning an Option, matching `Vec::set` — an index
    // that must be valid is a caller bug, whereas `char_at`'s query is not.
    pub fn slice(self, start: Int, end: Int) -> String {
        let cs = str_chars(self)
        if start < 0 { panic("String::slice start is negative") }
        if end > cs.len() { panic("String::slice end is past the end of the string") }
        if start > end { panic("String::slice start is after end") }
        chars_to_string(cs, start, end)
    }

    // Reverses CODEPOINTS, so a combining accent detaches from the character
    // it modifies. That is inherent to codepoint-level operations and is why
    // this module is not grapheme-aware (see the design doc, decision D1).
    pub fn reverse(self) -> String {
        let cs = str_chars(self)
        let n = cs.len()
        if n == 0 { return "" }
        let mut out = [' '; n]
        for i in 0..n { out[i] = cs[n - 1 - i] }
        str_from_chars(out)
    }
```

- [ ] **Step 7: Run the tests, including each panic path**

Run: `cargo build -p nova-cli`, then `cargo test -p nova-cli --test run_tests str_from_chars_round_trips`, `cargo test -p nova-runtime str_from_chars`, `cargo test --workspace --no-fail-fast`

Then check each of `slice`'s three panics by hand, since a panic aborts the process and cannot sit in the shared fixture:

```bash
printf 'fn main() { println("${\"abc\".slice(0 - 1, 2)}") }\n' > /tmp/s1.nova
```

Expected for the three cases (`start` negative, `end` past the end, `start > end`): exit non-zero with `nova: panic: String::slice …` and the matching message. Add one `#[test]` per case asserting the message, modelled on the existing `Vec::set` out-of-range test.

- [ ] **Step 8: Commit**

```bash
git add crates/ std/strings/lib.nova
git commit -m "feat(std): add str_from_chars, String::slice and String::reverse"
```

---

### Task 5 (S5): Search — `starts_with`, `ends_with`, `index_of`, `contains`

No new intrinsics. All four share one private matcher.

**Files:**
- Modify: `std/strings/lib.nova`
- Test: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `str_chars` (S3).
- Produces: private `chars_match_at(h: [Char], n: [Char], at: Int) -> Bool`; `String::starts_with`, `ends_with`, `index_of(self, needle: String) -> Option<Int>`, `contains(self, needle: String) -> Bool`.

- [ ] **Step 1: Write the failing test**

Per spec §4.2, an **empty needle matches everywhere**: `contains("") == true`, `index_of("") == Some(0)`, `starts_with("") == ends_with("") == true`. Indices are codepoints, so a multi-byte prefix must not shift them.

```rust
/// Search is codepoint-indexed, and an empty needle matches at position 0
/// (spec §4.2). `index_of` on "héllo wörld" must report 6 for "wörld", not a
/// byte offset — a byte-based implementation reports 7.
#[test]
fn string_search_is_codepoint_indexed_and_empty_needle_matches() {
    let src = "fn main() {\n\
               let s = \"héllo wörld\"\n\
               println(\"${s.index_of(\"wörld\").unwrap_or(0 - 1)} \
               ${s.index_of(\"zzz\").unwrap_or(0 - 1)} \
               ${s.index_of(\"\").unwrap_or(0 - 1)}\")\n\
               println(\"${s.starts_with(\"hé\")} ${s.starts_with(\"x\")} \
               ${s.ends_with(\"rld\")} ${s.ends_with(\"x\")}\")\n\
               println(\"${s.contains(\"ö\")} ${s.contains(\"q\")} \
               ${s.contains(\"\")} ${s.starts_with(\"\")} ${s.ends_with(\"\")}\")\n\
               println(\"${\"\".index_of(\"a\").unwrap_or(0 - 1)} \
               ${\"aaa\".index_of(\"aa\").unwrap_or(0 - 1)} \
               ${\"abc\".starts_with(\"abcd\")}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-search");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("6 -1 0\ntrue false true false\ntrue false true true true\n-1 0 false\n");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests string_search_is_codepoint_indexed`
Expected: FAIL — no `index_of` on `String`.

- [ ] **Step 3: Add the private matcher**

At the top level of `std/strings/lib.nova`:

```nova
// Whether `n` occurs in `h` starting exactly at `at`. A needle that would run
// past the end of `h` does not match; an empty needle matches at any `at`
// within `h`, including `h.len()`.
fn chars_match_at(h: [Char], n: [Char], at: Int) -> Bool {
    if at < 0 { return false }
    if at + n.len() > h.len() { return false }
    let mut k = 0
    while k < n.len() {
        if h[at + k] != n[k] { return false }
        k = k + 1
    }
    true
}
```

- [ ] **Step 4: Add the four methods**

```nova
    pub fn starts_with(self, prefix: String) -> Bool {
        chars_match_at(str_chars(self), str_chars(prefix), 0)
    }

    pub fn ends_with(self, suffix: String) -> Bool {
        let h = str_chars(self)
        let n = str_chars(suffix)
        if n.len() > h.len() { return false }
        chars_match_at(h, n, h.len() - n.len())
    }

    // Codepoint index of the FIRST occurrence, or `None`. An empty needle
    // occurs at every position, so it reports `Some(0)`.
    pub fn index_of(self, needle: String) -> Option<Int> {
        let h = str_chars(self)
        let n = str_chars(needle)
        if n.len() == 0 { return Some(0) }
        if n.len() > h.len() { return None }
        let last = h.len() - n.len()
        let mut at = 0
        while at <= last {
            if chars_match_at(h, n, at) { return Some(at) }
            at = at + 1
        }
        None
    }

    pub fn contains(self, needle: String) -> Bool { self.index_of(needle).is_some() }
```

- [ ] **Step 5: Run the tests**

Run: `cargo build -p nova-cli` then `cargo test -p nova-cli --test run_tests string_search_is_codepoint_indexed` and `cargo test --workspace --no-fail-fast`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add std/strings/lib.nova crates/nova-cli/tests/run_tests.rs
git commit -m "feat(std): add String starts_with, ends_with, index_of and contains"
```

---

### Task 6 (S6): `split` and `join`

The trickiest algorithms in the module, and the ones with the most spec-pinned edge cases.

**Files:**
- Modify: `std/strings/lib.nova`
- Test: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `chars_match_at` (S5), `chars_to_string` (S4), `str_len_chars` (S2).
- Produces: `String::split(self, sep: String) -> [String]`, `String::join(self, parts: [String]) -> String`.

- [ ] **Step 1: Write the failing test**

Every row of spec §4.2 that concerns `split` gets a line here:

```rust
/// `split`'s pinned semantics (spec §4.2): a missing separator yields a
/// one-element array and NEVER an empty one; adjacent, leading and trailing
/// separators produce empty strings with no collapsing; and an EMPTY
/// separator splits into single codepoints — the JavaScript behaviour, chosen
/// because Rust adds boundary empties and Python raises, so there is no
/// consensus to inherit. `join` hangs off the separator, not the parts.
#[test]
fn string_split_and_join_match_the_pinned_semantics() {
    let src = "fn main() {\n\
               let a = \"a,b,c\".split(\",\")\n\
               println(\"${a.len()} ${a[0]}${a[1]}${a[2]}\")\n\
               let b = \"abc\".split(\",\")\n\
               println(\"${b.len()} ${b[0]}\")\n\
               let c = \",a,\".split(\",\")\n\
               println(\"${c.len()} [${c[0]}][${c[1]}][${c[2]}]\")\n\
               let d = \"a,,b\".split(\",\")\n\
               println(\"${d.len()} [${d[0]}][${d[1]}][${d[2]}]\")\n\
               let e = \"a→b\".split(\"→\")\n\
               println(\"${e.len()} ${e[0]}${e[1]}\")\n\
               let f = \"abc\".split(\"\")\n\
               println(\"${f.len()} ${f[0]}|${f[1]}|${f[2]}\")\n\
               println(\"${\"\".split(\"\").len()} ${\"\".split(\",\").len()}\")\n\
               let g = \"xx\".split(\"xx\")\n\
               println(\"${g.len()} [${g[0]}][${g[1]}]\")\n\
               println(\"[${\",\".join(a)}] [${\"\".join(f)}] [${\"-\".join([])}]\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-split");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout(
            "3 abc\n1 abc\n3 [][a][]\n3 [a][][b]\n2 ab\n3 a|b|c\n0 1\n2 [][]\n\
             [a,b,c] [abc] []\n",
        );
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests string_split_and_join_match_the_pinned_semantics`
Expected: FAIL — no `split` on `String`.

- [ ] **Step 3: Implement `split`**

Two passes — count the pieces, then fill — so the result is a plain `[String]` and `std/strings` depends on `std/core` alone rather than on `Vec` from `std/collections` (design decision D4).

```nova
    // Split on every non-overlapping occurrence of `sep`, left to right.
    //
    // No separator found yields a ONE-element array holding the whole string,
    // never an empty array. Adjacent, leading and trailing separators each
    // produce an empty piece — nothing is collapsed or trimmed. An EMPTY
    // separator splits into single codepoints (the JavaScript behaviour;
    // Rust adds boundary empties and Python raises, so there is no consensus
    // to inherit), and `"".split("")` is `[]`.
    pub fn split(self, sep: String) -> [String] {
        let h = str_chars(self)
        let s = str_chars(sep)
        if s.len() == 0 {
            if h.len() == 0 { return [] }
            let mut single = [""; h.len()]
            for i in 0..h.len() { single[i] = chars_to_string(h, i, i + 1) }
            return single
        }
        // Pass 1: count the pieces. One more piece than separators found.
        let mut pieces = 1
        let mut i = 0
        while i + s.len() <= h.len() {
            if chars_match_at(h, s, i) {
                pieces = pieces + 1
                i = i + s.len()
            } else {
                i = i + 1
            }
        }
        // Pass 2: fill, walking the same way so the two passes agree.
        let mut out = [""; pieces]
        let mut w = 0
        let mut start = 0
        let mut j = 0
        while j + s.len() <= h.len() {
            if chars_match_at(h, s, j) {
                out[w] = chars_to_string(h, start, j)
                w = w + 1
                j = j + s.len()
                start = j
            } else {
                j = j + 1
            }
        }
        out[w] = chars_to_string(h, start, h.len())
        out
    }
```

- [ ] **Step 4: Implement `join`**

```nova
    // Join `parts` with `self` between them: `",".join(parts)`. The separator
    // is the receiver rather than this being a free `join(parts, sep)`,
    // because a top-level `pub fn` is glob-imported into every module and
    // would take the name `join` from all user code.
    pub fn join(self, parts: [String]) -> String {
        if parts.len() == 0 { return "" }
        let sep = str_chars(self)
        let mut total = sep.len() * (parts.len() - 1)
        for i in 0..parts.len() { total = total + str_len_chars(parts[i]) }
        if total == 0 { return "" }
        // Filler is overwritten in full below; `total > 0` here.
        let mut out = [' '; total]
        let mut w = 0
        for i in 0..parts.len() {
            if i > 0 {
                for k in 0..sep.len() {
                    out[w] = sep[k]
                    w = w + 1
                }
            }
            let p = str_chars(parts[i])
            for k in 0..p.len() {
                out[w] = p[k]
                w = w + 1
            }
        }
        str_from_chars(out)
    }
```

- [ ] **Step 5: Run the tests**

Run: `cargo build -p nova-cli` then `cargo test -p nova-cli --test run_tests string_split_and_join` and `cargo test --workspace --no-fail-fast`
Expected: PASS.

- [ ] **Step 6: Prove the two passes cannot disagree**

`split`'s correctness depends on both passes walking identically; if they diverge, pass 2 either writes past `out` (bounds abort) or leaves a slot as the `""` filler (silent wrong answer).

Temporarily change pass 2's `j = j + s.len()` to `j = j + 1` after a match, rebuild, and run the test.
Expected: it **FAILS** — overlapping matches make pass 2 find more separators than pass 1 counted. Restore, rebuild, confirm green. Report what you saw.

- [ ] **Step 7: Commit**

```bash
git add std/strings/lib.nova crates/nova-cli/tests/run_tests.rs
git commit -m "feat(std): add String::split and String::join"
```

---

### Task 7 (S7): `trim` family and `repeat`

**Files:**
- Modify: `std/strings/lib.nova`
- Test: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `str_chars` (S3), `chars_to_string` (S4).
- Produces: private `char_is_whitespace(c: Char) -> Bool`; `String::trim`, `trim_start`, `trim_end`, `repeat(self, n: Int) -> String`.

- [ ] **Step 1: Write the failing test**

```rust
/// The trim family and `repeat`. `repeat(0)` is "" and a negative count
/// panics (spec §4.2). Trimming an all-whitespace string yields "".
#[test]
fn string_trim_family_and_repeat() {
    let src = "fn main() {\n\
               let s = \"  héllo\\t\\n\"\n\
               println(\"[${s.trim()}][${s.trim_start()}][${s.trim_end()}]\")\n\
               println(\"[${\"   \".trim()}][${\"\".trim()}][${\"x\".trim()}]\")\n\
               println(\"[${\"ab\".repeat(3)}][${\"ab\".repeat(0)}][${\"\".repeat(5)}]\")\n\
               println(\"[${\"→\".repeat(2)}]\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-trim");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("[héllo][héllo\t\n][  héllo]\n[][][x]\n[ababab][][]\n[→→]\n");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests string_trim_family_and_repeat`
Expected: FAIL — no `trim` on `String`.

- [ ] **Step 3: Add the whitespace predicate**

At the top level of `std/strings/lib.nova`:

```nova
// Whether `c` is whitespace, for the `trim` family.
//
// DELIBERATE APPROXIMATION: this is an explicit list, not Unicode's full
// White_Space property. `Char` cannot currently be asked whether it is
// whitespace, and an exact answer would need a sixth permanent runtime ABI
// symbol, which is not worth it for this increment. It lives in exactly one
// place so there is one line to correct when that changes.
fn char_is_whitespace(c: Char) -> Bool {
    if c == ' ' { return true }
    if c == '\t' { return true }
    if c == '\n' { return true }
    if c == '\r' { return true }
    // The non-ASCII spaces are compared by SCALAR VALUE, not as character
    // literals: `'\u{00A0}'` DOES NOT LEX — the `\u{…}` escape works in a
    // string literal but not in a char literal (verified; it fails with
    // `L0001: unexpected character '`). `char_to_int` is already a std-only
    // builtin, so an std module can compare codepoints directly.
    let v = char_to_int(c)
    if v == 160 { return true }    // U+00A0 NO-BREAK SPACE
    if v == 8194 { return true }   // U+2002 EN SPACE
    if v == 8195 { return true }   // U+2003 EM SPACE
    if v == 12288 { return true }  // U+3000 IDEOGRAPHIC SPACE
    false
}
```

- [ ] **Step 4: Add the four methods**

```nova
    pub fn trim(self) -> String {
        let cs = str_chars(self)
        let a = trim_start_index(cs)
        chars_to_string(cs, a, trim_end_index(cs, a))
    }

    pub fn trim_start(self) -> String {
        let cs = str_chars(self)
        chars_to_string(cs, trim_start_index(cs), cs.len())
    }

    pub fn trim_end(self) -> String {
        let cs = str_chars(self)
        chars_to_string(cs, 0, trim_end_index(cs, 0))
    }

    // `n` copies of `self`. Builds one array rather than concatenating, both
    // because `String` has no `+` (E0013) and because repeated interpolation
    // would be quadratic.
    pub fn repeat(self, n: Int) -> String {
        if n < 0 { panic("String::repeat count must not be negative") }
        let cs = str_chars(self)
        let total = cs.len() * n
        if total == 0 { return "" }
        let mut out = [' '; total]
        let mut w = 0
        for r in 0..n {
            for i in 0..cs.len() {
                out[w] = cs[i]
                w = w + 1
            }
        }
        str_from_chars(out)
    }
```

with these two private helpers at the top level, so the two ends are defined once each:

```nova
// First index at or after 0 that is not whitespace; `cs.len()` if all of it is.
fn trim_start_index(cs: [Char]) -> Int {
    let mut a = 0
    while a < cs.len() {
        if char_is_whitespace(cs[a]) { a = a + 1 } else { return a }
    }
    cs.len()
}

// Exclusive end index of the last non-whitespace codepoint at or after
// `floor`; `floor` if everything from there on is whitespace.
fn trim_end_index(cs: [Char], floor: Int) -> Int {
    let mut b = cs.len()
    while b > floor {
        if char_is_whitespace(cs[b - 1]) { b = b - 1 } else { return b }
    }
    floor
}
```

- [ ] **Step 5: Run the tests, and check `repeat`'s panic**

Run: `cargo build -p nova-cli` then `cargo test -p nova-cli --test run_tests string_trim_family_and_repeat` and `cargo test --workspace --no-fail-fast`

Then add a `#[test]` asserting `"x".repeat(0 - 1)` exits non-zero with `nova: panic: String::repeat count must not be negative`, modelled on the existing `Vec::set` out-of-range test.

- [ ] **Step 6: Commit**

```bash
git add std/strings/lib.nova crates/nova-cli/tests/run_tests.rs
git commit -m "feat(std): add the String trim family and String::repeat"
```

---

### Task 8 (S8): `str_to_upper` / `str_to_lower` + the case-mapping methods

Two intrinsics in one task: they are the same shape, and the single test that matters (`ß → SS`) covers the reason both are whole-string.

**Files:**
- Modify: the six touchpoints, plus `std/strings/lib.nova`

**Interfaces:**
- Consumes: S2's wiring pattern.
- Produces: `str_to_upper(String) -> String`, `str_to_lower(String) -> String`, `RtFunc::StrToUpper`/`StrToLower`, `nova_rt_str_to_upper`/`_lower`, `String::to_upper`/`to_lower`.

- [ ] **Step 1: Write the failing test**

```rust
/// Case mapping is WHOLE-STRING, not `Char -> Char`, because `ß` uppercases
/// to the two characters `SS`. A `Char -> Char` implementation cannot express
/// that and would silently corrupt such input — so `"ß".to_upper().len()`
/// being 2 is the assertion that proves the signature choice.
#[test]
fn string_case_mapping_is_whole_string_not_per_char() {
    let src = "fn main() {\n\
               println(\"${\"Straße\".to_upper()} ${\"ß\".to_upper().len()}\")\n\
               println(\"${\"HÉLLO WÖRLD\".to_lower()} ${\"\".to_upper()}|\")\n\
               println(\"${\"İ\".to_lower().len()} ${\"abc123\".to_upper()}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-case");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("STRASSE 2\nhéllo wörld |\n2 ABC123\n");
}
```

Note `"İ"` (U+0130) lowercases to two codepoints in Rust, which is why its expected length is 2. If Rust's actual output differs on this platform, **verify with a tiny Rust check before adjusting the fixture** — do not just paste whatever was printed.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests string_case_mapping_is_whole_string`
Expected: FAIL — no `to_upper` on `String`.

- [ ] **Step 3: Add the runtime functions**

```rust
/// Uppercase `s` with full Unicode case mapping.
///
/// Whole-string rather than `Char` → `Char` because the mapping is not 1:1 —
/// `ß` uppercases to `SS` — so a per-character signature could not express it
/// and would silently corrupt such input.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_to_upper(s: *const NovaStr) -> *mut NovaStr {
    gc_str(&as_str(s).to_uppercase())
}

/// Lowercase `s` with full Unicode case mapping. Whole-string for the same
/// reason as [`nova_rt_str_to_upper`].
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_to_lower(s: *const NovaStr) -> *mut NovaStr {
    gc_str(&as_str(s).to_lowercase())
}
```

Plus both `symbols()` entries.

- [ ] **Step 4: Add a runtime unit test**

```rust
    #[test]
    fn case_mapping_handles_the_non_one_to_one_cases() {
        unsafe {
            assert_eq!(as_str(nova_rt_str_to_upper(make_str("Straße"))), "STRASSE");
            assert_eq!(as_str(nova_rt_str_to_lower(make_str("HÉLLO"))), "héllo");
            assert_eq!(as_str(nova_rt_str_to_upper(make_str(""))), "");
        }
    }
```

- [ ] **Step 5: Wire both builtins**

```rust
    /// `str_to_upper(s: String) -> String` — full Unicode uppercase. Backs
    /// `std/strings`' `String::to_upper`. Whole-string rather than
    /// `Char` → `Char` because `ß` → `SS` is not 1:1. Std-only.
    StrToUpper,
    /// `str_to_lower(s: String) -> String` — full Unicode lowercase, for the
    /// same reason as [`Builtin::StrToUpper`]. Std-only.
    StrToLower,
```
```rust
            Builtin::StrToUpper => "str_to_upper",
            Builtin::StrToLower => "str_to_lower",
```
```rust
        Builtin::StrToUpper | Builtin::StrToLower => (vec![Ty::String], Ty::String),
```
```rust
    /// `(str) -> str` — full Unicode uppercase.
    StrToUpper,
    /// `(str) -> str` — full Unicode lowercase.
    StrToLower,
```
```rust
            RtFunc::StrToUpper => "nova_rt_str_to_upper",
            RtFunc::StrToLower => "nova_rt_str_to_lower",
```
```rust
            RtFunc::StrToUpper | RtFunc::StrToLower => (vec![MirTy::Ptr], MirTy::Ptr),
```
```rust
                    Builtin::StrToUpper => Some(RtFunc::StrToUpper),
                    Builtin::StrToLower => Some(RtFunc::StrToLower),
```

`STD_ONLY` reaches its final `[Builtin; 8]`; add both to the `hint` match.

- [ ] **Step 6: Add the Nova methods**

```nova
    // Full Unicode case mapping, so the result may be LONGER than the input
    // in codepoints: `"ß".to_upper()` is `"SS"`.
    pub fn to_upper(self) -> String { str_to_upper(self) }

    pub fn to_lower(self) -> String { str_to_lower(self) }
```

- [ ] **Step 7: Verify `STD_ONLY` is complete and the count is right**

Run: `cargo test -p nova-resolver --no-fail-fast`
Expected: PASS. `no_std_only_builtin_is_a_reserved_word` loops over `STD_ONLY`, so all five new builtins are automatically checked for not being reserved words in user code, and `std_only_builtins_are_visible_inside_std_modules` checks all three std modules can see them.

- [ ] **Step 8: Run everything**

Run: `cargo build -p nova-cli`, `cargo test --workspace --no-fail-fast`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`
Expected: all green.

- [ ] **Step 9: Commit**

```bash
git add crates/ std/strings/lib.nova
git commit -m "feat(std): add whole-string Unicode case mapping to std/strings"
```

---

### Task 9 (S9): Fix `Debug for String`, and correct the comments this design falsifies

The defect that motivated the phase. Also corrects two comments that stop being true — the failure mode the *previous* branch actually shipped, so treat it as part of the work, not a nicety.

**Files:**
- Modify: `std/core/lib.nova:157-178`, `crates/nova-typeck/src/check.rs` (the `hint` comment)
- Test: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `str_chars` (S3) — a builtin, so std/core uses it directly with **no** dependency on std/strings.
- Produces: `Debug for String` yielding a valid Nova literal.

- [ ] **Step 1: Write the failing test**

```rust
/// `("a\"b").dbg()` used to produce `"a"b"`, which is not a valid Nova
/// literal — the defect that motivated Phase 2.2b. Escaping needs to inspect
/// the string's contents, which `str_chars` now allows.
#[test]
fn debug_for_string_escapes_into_a_valid_literal() {
    let src = "fn main() {\n\
               println(\"${(\"a\\\"b\").dbg()}\")\n\
               println(\"${(\"back\\\\slash\").dbg()}\")\n\
               println(\"${(\"tab\\there\").dbg()}\")\n\
               println(\"${(\"\").dbg()} ${(\"é→\").dbg()}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-dbg");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("\"a\\\"b\"\n\"back\\\\slash\"\n\"tab\\there\"\n\"\" \"é→\"\n");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests debug_for_string_escapes`
Expected: FAIL, with `"a"b"` where `"a\"b"` was expected.

- [ ] **Step 3: Share the per-character escape, then use it for both impls**

`Debug for Char` already holds this logic. Do **not** write it a second time — the duplicated `Map::get`/`remove` probe scan from Phase 2.2a is the cautionary precedent. Add one private top-level helper in `std/core/lib.nova` and have both impls call it:

```nova
// The escape sequence for `c` inside a string literal, or `None` when `c`
// needs none. Quoting differs between a `Char` literal and a `String`
// literal, so the quote characters are the callers' business: `Debug for
// Char` escapes `'`, `Debug for String` escapes `"`, and both need the rest
// of this identically.
fn escape_common(c: Char) -> Option<String> {
    if c == '\\' { return Some("\\\\") }
    if c == '\n' { return Some("\\n") }
    if c == '\t' { return Some("\\t") }
    if c == '\r' { return Some("\\r") }
    if c == '\0' { return Some("\\0") }
    None
}
```

Rewrite `impl Debug for Char` to use it, keeping its existing behaviour exactly (it escapes `'` and not `"`), and add:

```nova
impl Debug for String {
    fn dbg(self) -> String {
        let cs = str_chars(self)
        let mut out = "\""
        for i in 0..cs.len() {
            let c = cs[i]
            if c == '"' {
                out = "${out}\\\""
            } else {
                match escape_common(c) {
                    Some(e) => out = "${out}${e}",
                    None => out = "${out}${c}",
                }
            }
        }
        "${out}\""
    }
}
```

Interpolation is used rather than `str_from_chars` because the pieces are strings of differing length, and `String` has no `+`. This is quadratic in the string's length; `dbg` is a diagnostic path, and stating that in the comment is the right trade rather than pretending otherwise.

If `match` on `Option<String>` with a bound payload does not compile in an std module, fall back to `is_some()`/`unwrap()` and note why.

- [ ] **Step 4: Correct the two comments this design falsifies**

`std/core/lib.nova:168-177` currently says escaping "needs a new compiler builtin … a `str_escape` in `Builtin::STD_ONLY`, backed by a runtime `nova_rt_str_escape`". That prediction is now wrong — no such symbol was added. Replace it with what actually happened: `str_chars` (added for `std/strings`) lets std/core do the escaping itself, so `Debug for String` needs no dedicated ABI symbol and no dependency on `std/strings`.

`crates/nova-typeck/src/check.rs`, in the `hint` match's comment, says the std-only builtins "are called from one hand-written std/core site each". With eight of them called from two std modules and several call sites each, that is no longer true. Reword it.

- [ ] **Step 5: Run the tests**

Run: `cargo build -p nova-cli`, then `cargo test -p nova-cli --test run_tests debug_for_string_escapes`, then `cargo test --workspace --no-fail-fast`
Expected: PASS. Watch for existing `Debug`/`dbg` tests and the `std_core.nova` gate — if the gate's output changes, that is expected only if it debug-prints a string; check the fixture and update it deliberately, explaining the change.

- [ ] **Step 6: Commit**

```bash
git add std/core/lib.nova crates/nova-typeck/src/check.rs crates/nova-cli/tests/run_tests.rs
git commit -m "fix(std): escape string contents in Debug for String"
```

---

### Task 10 (S10): The phase gate and CHANGELOG

**Files:**
- Create: `tests/runtime/strings.nova`, `tests/runtime/strings.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`, `CHANGELOG.md`

**Interfaces:**
- Consumes: the whole surface from S1–S9.
- Produces: three gate tests (`strings_run`, `strings_build_standalone`, `strings_under_gc_stress`).

- [ ] **Step 1: Write the fixture program**

Create `tests/runtime/strings.nova` covering **every** numbered item of the spec's §7 and **every row** of its §4.2 table. Nothing in it may panic — a panic aborts the process and truncates the remaining output, so the panic paths stay in their own `#[test]`s from S4 and S7. Cover at minimum:

1. Byte length ≠ codepoint length: `"café".len()` is 4, `"日本語".len()` is 3, plus a 4-byte scalar (`"🦀".len()` is 1).
2. `"Straße".to_upper()` is `STRASSE`, and `"ß".to_upper().len()` is 2.
3. `("a\"b").dbg()` and a backslash case.
4. Round-trip: for ASCII, accented, CJK and emoji input, `str_from_chars(str_chars(s))` equals `s` — exercised through Nova as `s.chars()` rebuilt via `"".join` of single-char slices, or by comparing `s.slice(0, s.len())` to `s`.
5. `chars()`'s array read back: `.len()` and individual elements.
6. Empty and single-codepoint cases for every scanning method: `"".split(",")`, `"".trim()`, `"".reverse()`, `slice(0, 0)`, `repeat(0)`, `",".join([])`.
7. `split` with a separator absent / leading / trailing / repeated / equal to the whole string / empty.
8. `slice`'s half-open boundary at both ends: `slice(0, 0)`, `slice(0, len)`, `slice(len, len)`.

- [ ] **Step 2: Generate the expected output, then read it critically**

```bash
cargo build -p nova-cli
./target/debug/nova.exe run tests/runtime/strings.nova > tests/runtime/strings.stdout
```

**Do not stop here.** Read every line and confirm it is what you decided in advance it should be. A fixture generated from actual output pins whatever the code does, including bugs. Any line you cannot justify from the spec is a finding to investigate, not to record.

- [ ] **Step 3: Add the three gate tests**

Model them exactly on `collections_run` / `collections_build_standalone` / `collections_under_gc_stress` in `crates/nova-cli/tests/run_tests.rs`, including the `.replace("\r\n", "\n")` normalisation:

```rust
/// `std/strings` end-to-end gate (Phase 2.2b). Every index in the module is a
/// codepoint, so a byte-based regression shows up as a wrong number here.
#[test]
fn strings_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/strings.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/strings.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture through the object-file backend.
#[test]
fn strings_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/strings.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/strings.nova", "strings");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// The same fixture with `NOVA_GC_STRESS=1` (collect on every allocation).
/// This is the reason the gate exists: `str_chars` and `str_from_chars`
/// introduce two new allocation shapes reachable from a builtin — a scanned
/// array of scalars, and a leaf byte buffer plus a scanned header — and the
/// intermediate `[Char]` must stay live across the allocations that follow
/// it. A missed root here means silently wrong text, not a crash.
#[test]
fn strings_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/strings.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let expected_clone = expected.clone();
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/strings.nova"))
        .assert()
        .success()
        .stdout(expected_clone);
    let _ = expected;
}
```

Drop the `expected_clone` dance if `stdout(expected)` compiles directly, as it does in the collections tests — match whatever those do rather than inventing a variation.

- [ ] **Step 4: Run all three modes**

Run: `cargo test -p nova-cli --test run_tests strings_ --no-fail-fast`
Expected: all three PASS. If `strings_under_gc_stress` fails while `strings_run` passes, that is a **collector** finding — an intermediate `[Char]` is being collected while still live. Report it; do not paper over it by restructuring the fixture.

- [ ] **Step 4b: Write the two `std/collections` panic tests that a comment already claims exist**

Not strings work, but it belongs here because this is the task that touches `run_tests.rs`, and leaving it means the next reader trusts a false statement.

`crates/nova-cli/tests/run_tests.rs:1207-1208` says, of the collections gate: "Nothing in here panics: `panic` aborts the process, which would truncate the remaining output. **`Vec::set` out of range and `unwrap` on the wrong variant have their own committed tests.**" Those tests **do not exist** — `git grep` finds only that comment. Phase 2.2a shipped a comment vouching for coverage that was never written, which is exactly the documented-but-unenforced pattern the preceding branch existed to eliminate. Two real panic paths in `std/collections` therefore have zero coverage.

Write them, rather than weakening the comment to match reality — the comment names the right tests, they were just never added. Model both on `panic_aborts_with_message` (`crates/nova-cli/tests/run_tests.rs:1333`), which is the file's actual idiom for a process-aborting program, and give each its own temp directory (`nova-collections-setoob`, `nova-collections-unwrap`).

```rust
/// `Vec::set` past the end aborts with its own message rather than
/// corrupting memory. The collections gate's doc comment has claimed this
/// test exists since Phase 2.2a; it did not, so the path was uncovered.
#[test]
fn vec_set_out_of_range_aborts_with_message() {
    let src = "fn main() {\n\
               let mut v: Vec<Int> = Vec::new()\n\
               v.push(1)\n\
               v.set(5, 9)\n\
               }";
    // … house temp-dir idiom, then assert failure() and that stderr
    // contains "Vec::set index out of range"
}

/// `unwrap` on a `None` aborts with its own message. Same provenance as
/// above — claimed by a comment, never written.
#[test]
fn unwrap_on_the_wrong_variant_aborts_with_message() {
    let src = "fn main() {\n\
               let o: Option<Int> = None\n\
               println(\"${o.unwrap()}\")\n\
               }";
    // … assert failure() and the message std/core's `unwrap` actually panics with
}
```

**Read the real panic messages out of `std/collections/lib.nova` and `std/core/lib.nova` before asserting them** — do not guess the wording, and do not change the messages to match a guess. If either path turns out **not** to panic (for instance if `Vec::set`'s guard is missing, or `unwrap`'s message differs from what the comment implies), that is a genuine finding in previously shipped code: report it rather than adjusting the test to pass.

- [ ] **Step 5: Update the CHANGELOG**

Under `[Unreleased]`, record: the five new intrinsics and that they are std-only; `std/strings` as the third embedded std module with its 18 methods; that `String` now has **18 inherent methods that shadow same-named user trait methods on `String`** (not an error — inherent wins by priority — but permanent); the codepoint-not-bytes contract; the `Debug for String` escaping fix; and the deliberate limitations (approximate whitespace set, O(n) allocation per inspection, no `replace`/padding/parsing).

- [ ] **Step 6: Final verification**

Run all four, and record the actual output of each:

```bash
cargo test --workspace --no-fail-fast
```
```bash
cargo clippy --all-targets --all-features -- -D warnings
```
```bash
cargo fmt --check
```

Plus both pre-existing gates by hand in all three modes, confirming they are byte-identical to before the branch:

```bash
for f in collections std_core strings; do diff <(tr -d '\r' < tests/runtime/$f.stdout) <(./target/debug/nova.exe run tests/runtime/$f.nova | tr -d '\r') && echo "$f OK"; done
```

- [ ] **Step 7: Commit**

```bash
git add tests/runtime/strings.nova tests/runtime/strings.stdout crates/nova-cli/tests/run_tests.rs CHANGELOG.md
git commit -m "test(std): add the std/strings end-to-end gate under all three backends"
```

---

## Plan Self-Review

**Spec coverage** — every section of the design maps to a task:

| Spec | Task |
|---|---|
| §3 five intrinsics | S2 (`str_len_chars`), S3 (`str_chars`), S4 (`str_from_chars`), S8 (`str_to_upper`/`_lower`) |
| §3.1 six touchpoints | S2 Step 5 in full; S3/S4/S8 reference it |
| §3.2 array-layout hazard | S3 Steps 3, 4 and **8** (the deliberate break) |
| §3.3 invalid scalars | S4 Step 3 + its surrogate unit test |
| §4 the 18-method surface | S1 (`is_empty`), S2 (`len`), S3 (`chars`, `char_at`), S4 (`slice`, `reverse`), S5 (4 search), S6 (`split`, `join`), S7 (`trim`×3, `repeat`), S8 (`to_upper`, `to_lower`) — 18 total |
| §4.2 the eleven pinned cases | S4 (slice bounds + panics), S5 (empty needle ×3), S6 (split ×5), S7 (negative repeat), S3 (negative `char_at`), S4 (`reverse`) |
| §4.3 whitespace | S7 Step 3 |
| §4.4 coherence + one block | S1 Step 3's module header |
| §5 `Debug for String` | S9 |
| §6 accepted costs | S1's performance contract; CHANGELOG in S10 Step 5 |
| §7 gate | S10 |
| §9 risks | S3 Step 8, S6 Step 6, S10 Step 4 |
| §10 definition of done | S10 Step 6 |

**Method count check:** `is_empty`, `len`, `chars`, `char_at`, `slice`, `contains`, `starts_with`, `ends_with`, `index_of`, `split`, `trim`, `trim_start`, `trim_end`, `to_upper`, `to_lower`, `repeat`, `reverse`, `join` = **18**, matching spec §4 and §6.

**Type consistency check:** `str_chars`/`str_from_chars` are `[Char]` ⇄ `String` in every task. `chars_match_at(h, n, at)` has the same three-parameter order at all four call sites in S5 and both in S6. `chars_to_string(cs, start, end)` is `end`-exclusive at every call site (S4, S6, S7). `trim_start_index(cs)` / `trim_end_index(cs, floor)` are both introduced in S7 Step 4 and used only there. `index_of` returns `Option<Int>` and `contains` consumes it via `.is_some()`.

**`STD_ONLY` count progression:** 3 → 4 (S2) → 5 (S3) → 6 (S4) → 8 (S8). Every task that adds a builtin states the new length annotation, because the array is length-typed and will not compile otherwise.

**Every Nova construct in this plan was run against this compiler before the plan was written**, not assumed: private non-`pub` top-level helpers taking `[Char]`; `impl String` with an array parameter; `self == ""`; `[' '; n]` and `[""; n]` at runtime length; nested `for` mutating an outer counter; the empty range `0..0`; `break` inside a `while` inside a method; and S9's `match` on an `Option<String>` with a bound payload.

Three of those probes changed the plan:

1. **`String + String` is `E0013`** — there is no `+` for strings. Every result is therefore built through `str_from_chars` over a `[Char]`, which is *why* `str_from_chars` is a primitive rather than a convenience, and why `repeat` and `join` accumulate into an array instead of concatenating.
2. **`'\u{00A0}'` does not lex.** The `\u{…}` escape works in a *string* literal but not a *char* literal, so `char_is_whitespace` compares scalar values via `char_to_int` (S7 Step 3). Writing the obvious thing would have failed at S7 with a lexer error and no clue why.
3. ~~A char literal inside a `${…}` interpolation hole does not lex either.~~ **This was WRONG and is retracted.** Char literals inside a hole lex fine: `println("eq=${cs[0] == 'h'} lit=${'x'}")` prints `eq=true lit=x`. The original probe put `'\u{00A0}'` *inside* a hole, so the line failed at the `'` of the char literal and the failure was misattributed to the hole rather than to the `\u{…}` escape — finding 2 above is the real and only constraint. Test and fixture programs may compare and interpolate char literals directly; there is no need to bind to a local first.

**Remaining plan risk, stated rather than hidden:** the `to_upper`/`to_lower` expectations in S8 Step 1 include `"İ".to_lower().len()`, whose value depends on Rust's Unicode tables. The step says to verify it with a standalone Rust check before adjusting the fixture rather than pasting whatever was printed — the one place in this plan where the expected output is not derivable from the spec alone.
