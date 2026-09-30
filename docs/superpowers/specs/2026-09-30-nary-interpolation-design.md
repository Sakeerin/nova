# Lower string interpolation to one n-ary concatenation — design

**Date:** 2026-09-30
**Branch:** `nary-interpolation`
**Base:** `main` == `origin/main` == `0e00190`, 735 commits. The last full
suite was 1141 passed / 0 failed / 8 ignored across 45 targets, on the PR
#60 branch whose tree `0e00190` carries.

---

## 1. What this is, in one paragraph

String interpolation is lowered to a chain of pairwise concatenations. An
interpolation of *n* parts makes *n* − 1 calls to `nova_rt_str_concat`,
each building a new two-object string that copies everything built so far.
`examples/05-json-api/BENCHMARK.md` measured `user_json`'s seven-part
interpolation at 18 objects per call after PR #60. **This change lowers any
interpolation of three or more parts to one heap array of the parts and one
call to a new `nova_rt_str_concat_n`**, which copies each part once and
builds one string. That is predicted to cut the probe to 9 objects, and it
applies to every interpolation in every program. Output is byte-identical.

---

## 2. What is known, and from where

### 2.1 The costs (measured on `0e00190` plus scratch counters)

Every runtime string is two GC objects: a 16-byte `NovaStr` header and a
separate byte buffer (`gc_str` in `crates/nova-runtime/src/lib.rs`).
Evaluating a string literal allocates one header pointing at static bytes
(`nova_rt_str_new`). With those two rules, every probe's count is exact:

| probe | objects per call | how |
|---|---|---|
| `"{\"id\":${a},\"name\":${x},\"email\":${y}}"` (7 parts: 4 literals, an `Int`, 2 strings) | 17.990 | 4 literal headers + 2 for the `Int` conversion + 6 pairwise concatenations × 2 = 18 |
| `"${a}"` (one `Int`) | 1.993 | one conversion = 2 |
| `"${x}${y}"` (two strings) | 1.993 | one concatenation = 2 |

### 2.2 The path (read from source on `0e00190`)

- The type checker desugars interpolation to `hir::ExprKind::StrConcat`
  over `ToStr` conversions (`crates/nova-typeck/src/check.rs`,
  `crates/nova-hir/src/lib.rs`'s module doc).
- `crates/nova-mir/src/lower.rs`, `K::StrConcat`, folds the parts left
  into `CallRuntime { func: RtFunc::StrConcat, args: [acc, rhs] }`, one per
  part after the first.
- Runtime calls have a fixed signature per `RtFunc`
  (`RtFunc::signature`). Both code generators emit them generically from
  that table, and neither mentions `StrConcat`. So a variadic call is not
  available, and a single-argument call is.
- The MIR can build an array (`Stmt::MakeArray`, a `{ len, elements… }`
  heap object). `nova_rt_str_from_chars` already reads that layout from the
  runtime side.
- No string `+` was found in the type checker. The only producer of
  `hir::ExprKind::StrConcat` found is interpolation's desugaring.

---

## 3. The change

### 3.1 Lowering (`crates/nova-mir/src/lower.rs`, `K::StrConcat`)

| parts | lowering |
|---|---|
| 0 | unchanged: an empty literal |
| 1 | unchanged: the part itself |
| 2 | unchanged: one `CallRuntime(StrConcat)` |
| 3 or more | each part lowered into a temp in source order, as today; then one `Stmt::MakeArray` of those temps as `MirTy::Ptr`; then one `CallRuntime(StrConcatN, [array])` |

Parts are lowered exactly as before, in the same order, so evaluation order,
side effects and any `.await` inside a part are unchanged. Only the joining
changes. The two-part case keeps the pairwise call because there the array
would cost more (3 objects against 2).

### 3.2 The MIR table (`crates/nova-mir/src/lib.rs`)

- A new `RtFunc::StrConcatN`, named `"nova_rt_str_concat_n"`, with signature
  `(vec![MirTy::Ptr], MirTy::Ptr)`.

### 3.3 The runtime (`crates/nova-runtime/src/lib.rs`)

- A new `#[no_mangle] pub unsafe extern "C" fn nova_rt_str_concat_n(parts:
  *const u8) -> *mut NovaStr`.
- It reads the array's length at offset 0 and the part `i` pointer at byte
  offset `8 + 8*i`, as `nova_rt_str_from_chars` does. It sums the parts'
  byte lengths, then builds a `String` of that capacity by `push_str` of
  each part, and returns `gc_str(&s)`.
- **GC safety.** Every part's bytes are copied into the Rust `String`
  before the function's first GC allocation, in `gc_str`. So a collection
  triggered while building the result cannot free anything the function
  still reads. The pairwise `nova_rt_str_concat` relies on the same
  argument.
- A negative length is treated as zero, as `nova_rt_str_from_chars` does.
- It is registered in `symbols()`.

### 3.4 Unchanged

- **Both code generators.**
- **`nova_rt_str_concat`**, still used for two-part interpolations.
- **The type checker's desugaring**, and so the HIR module doc, which
  remains true.
- **The `ToStr` conversions.**
- **The one header each string literal allocates when evaluated.**

---

## 4. Testing

The three-part lowering test in 1 and the runtime tests in 2 are shown
failing against the unfixed code first; the runtime tests do not compile
before the change. The two-part lowering test in 1 and the output tests in 3
are guards. They pass before and after by design, because the two-part path
and the output must not change, and their power is shown by the mutations
in 4.

1. **MIR lowering** (`crates/nova-mir/tests/lower_tests.rs`).
   - A three-part interpolation lowers to exactly one `Stmt::MakeArray` and
     one `CallRuntime(StrConcatN)`, and no `CallRuntime(StrConcat)`.
   - A two-part interpolation lowers to exactly one `CallRuntime(StrConcat)`
     and no `StrConcatN`.
2. **The runtime function** (a `nova-runtime` unit test). On hand-built
   arrays of 0, 1, 3 and 5 parts, it returns the concatenation. That
   includes an empty part and multi-byte UTF-8 parts, where byte and
   character lengths differ.
3. **End to end:** a new `tests/runtime/interpolation_nary.nova` fixture
   with a `.stdout` file, and `nova-cli` tests running it normally and
   under `NOVA_GC_STRESS=1`. The fixture covers:
   - interpolations of 3 to 8 parts mixing `Int`, `Float`, `Bool`, `Char`,
     a user `Display` type, empty strings and non-ASCII text;
   - parts with side effects, printed in order, to pin evaluation order;
   - an `.await` inside a part of an `async fn`'s interpolation, so the
     part temps survive a suspension;
   - an interpolation inside a loop, so repeated evaluation is exercised.
4. **Mutations, each run and its outcome recorded:**
   - `nova_rt_str_concat_n` skipping its last part must fail 2 and 3;
   - reversing the part order must fail 2 and 3;
   - applying the n-ary path to two parts must fail 1.

   A mutant that hangs is recorded as hanging, not as failing.
5. **Must stay green:**
   - the existing interpolation-bearing fixtures (their output must be
     byte-identical);
   - `every_rt_func_symbol_is_registered_with_the_jit`;
   - the existing MIR test for `.await` inside an interpolation;
   - the full workspace suite, clippy `--all-targets --all-features -D
     warnings`, fmt, and CI on all three OSes.

---

## 5. Measurement

With the scratch counters, which are never committed:

- **Objects per call, before against after:** the 7-part probe (predicted
  18 → 9), `user_json`, and `users_json` at ten users.
- **Per request:** `examples/05-json-api`'s objects per ten-user request.
  The prediction is written into the record *before* the run. It is
  derived from `user_json` alone (ten calls of 7 parts, so −9 each), plus
  a count of the three-or-more-part interpolations `std/http` and the
  example execute per request. The code is read for that count, not
  guessed.
- **Throughput:** ten users, alternated, three fresh processes per build,
  one fresh process per reading, with ranges and each binary's byte size.

---

## 6. Records

- **`examples/05-json-api/BENCHMARK.md`:** a dated amendment with §5's
  figures, and a pointer to it from the "(json-drain-no-option)"
  amendment's list of what remains.
- **`CHANGELOG.md`:** a `### Changed` entry under `[Unreleased]`.
- **Every tracked document that describes how interpolation is lowered or
  what it costs**, found by a set difference (every tracked file naming
  `StrConcat`, `nova_rt_str_concat` or interpolation's lowering, minus the
  files the branch touched). Each is amended where it is now wrong, or
  recorded as deliberately left.
- **Then a PR, merged by rebase.**

---

## 7. Out of scope

- **Static literal headers**, which would make evaluating a literal allocate
  nothing.
- **One-allocation strings**, with the bytes inline in the header.
- **`quote` without a `Vec`** in `std/json`.
- **String `+`**, which was not found in the type checker.
- **Folding adjacent literal parts at compile time.** It would save headers,
  but it is a separate optimisation.
