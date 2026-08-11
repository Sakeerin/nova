# `std/fs` on Strings — Design

**Status:** approved 2026-08-11. Increment 1 of 4 in the decomposition of `std/fmt` + `std/io`, the
items sub-phase 2.1 cut pending async.

**Base:** `main` at `878082e` (async core plus its task-identity, reserved-names and park-set
follow-ups merged and pushed; 847 tests, 8 deliberately ignored). Note that review's item 5 — a
`BlockId` newtype, a heap-valued output in the async gate fixture, an async case in the LLVM
`--release` run, and three smaller items — is **not** done; "all the follow-ups are closed" would be
an overclaim.

---

## 1. Why this, and why now

Nova has no file I/O. `std/fmt` + `std/io` was cut from 2.1 because the **I/O operations** in
`nova-spec/20-STDLIB.md` §4 and §5 are `async fn` and async did not exist; async and a park set now
do. Note the scope of that claim: §3's `print` family are blocking writes and are plain `fn`s in the
spec, and §4's `stdin`/`stdout`/`stderr` constructors are not async either — so "every blocking
operation in the stdlib spec is async" would be false. What is async is reading, writing, flushing
and the filesystem calls.

But the spec surface is **not** buildable as one increment, and the reason is specific: §4's
signatures are Rust-shaped, and that shape — not the I/O — is what requires language work.
`read(self, buf: &mut [u8])` is a **buffer-filling** API needing both references and a byte type.
Nova is garbage-collected, where a **buffer-returning** signature needs neither. So the reference
requirement may be avoidable permanently rather than scheduled, and that question should be settled
before anything is built against the Rust-shaped form.

This increment therefore takes the part that needs **no language work at all**: the `String`-and-`Bool`
subset of §5 `std/fs`, plus the two missing members of §3's print family. It is also the useful
majority of file I/O for a language at this stage.

### 1.1 The decomposition this belongs to

| # | Increment | Needs |
|---|---|---|
| **1** | **`std/fs` on Strings, `eprint`/`eprintln` — this spec** | **nothing new** |
| 2 | A byte type (`u8` or `Bytes`) | lexer, typeck, MIR, both backends, GC scan decisions |
| 3 | `Read`/`Write`, `stdin`/`stdout`/`stderr`, `File` | #2, plus concrete types in place of `impl Trait` |
| 4 | True non-blocking I/O: a poller, `Wait::Io`, timed waits | #3 |

## 2. What is established, and how

**The right-hand column is how each was established**, because this project has repeatedly shipped a
claim measured on one shape and stated for all of them, and because a claim recalled from memory
arrives without the hedging its original measurement had.

| Claim | How established |
|---|---|
| `u8` is not a type: `fn f(x: u8)` is `E0001 cannot find type 'u8'` | **measured** — `nova check` |
| `&Int` is `E0900 reference and pointer types are not supported yet` — parses, rejected in typeck | **measured** |
| `impl Trait` in return position **does not parse**: `P0001 expected type (in return type), found 'impl'` | **measured** |
| `[Int]` arrays work end to end | **measured** |
| `eprint`/`eprintln` do not exist (`E0001 cannot find function`) | **measured** |
| `print`/`println`/`panic` are `Builtin::GLOBAL`, `[Builtin; 3]` — builtins, not Nova source | read |
| `Builtin::STD_ONLY` is `[Builtin; 17]` | read |
| `STD_MODULES` is `[(&str, &str); 4]`: core, collections, strings, task | read |
| No `nova_rt_*` file or I/O symbols exist yet | read |

**Recorded in memory but NOT re-derived for this spec — re-derive before relying on any of them:**

- `String` fails `require_ffi_safe`, so user-level `extern` cannot take a path; this needs `STD_ONLY`
  intrinsics in the pattern `std/strings` established.
- `str_chars` is the existing precedent for a runtime function that **constructs a Nova array**, and
  it is pinned by asserting the tracked `(size, scan)` through `gc::object_info` because reproducing
  codegen's `{len, elems at 8+8i}` layout wrongly is a silent miscompile.
- Records are heap objects passed by pointer, so a record built in Nova and handed to a builtin is
  the same object.

## 3. The change

### 3.1 `std/io` — error types only

A fifth `STD_MODULES` entry holding nothing but the error surface, so §4's traits have a home to
arrive into later.

```nova
pub record IoError { pub kind: IoErrorKind   pub message: String }

pub type IoErrorKind =
    | NotFound | PermissionDenied | AlreadyExists | InvalidData
    | Interrupted | TimedOut | ConnectionRefused | Other
```

A `STD_MODULES` entry is compiled as an implicit module and glob-imported into every user module —
stated in `std/task/lib.nova`'s own header and in ADR 0004 for `std/core`, i.e. **read for two of the
four entries, not verified for all**. If it holds for all of them, **which std module a type lives in
is not user-visible** and relocating these later would break no user code, so following §4's layout
costs nothing. Confirm it before relying on the relocation argument.

`AlreadyExists` and `InvalidData` are **additions to the spec's list** (§7).

### 3.2 `std/fs` — eight functions and one record

A sixth `STD_MODULES` entry.

```nova
pub async fn read_to_string(path: String) -> Result<String, IoError>
pub async fn write_string(path: String, content: String) -> Result<(), IoError>
pub async fn exists(path: String) -> Bool
pub async fn create_dir(path: String) -> Result<(), IoError>
pub async fn create_dir_all(path: String) -> Result<(), IoError>
pub async fn remove_file(path: String) -> Result<(), IoError>
pub async fn remove_dir_all(path: String) -> Result<(), IoError>
pub async fn read_dir(path: String) -> Result<[DirEntry], IoError>

pub record DirEntry { pub name: String   pub path: String   pub is_file: Bool   pub is_dir: Bool }
```

`exists` returns `Bool` rather than `Result`, as §5 specifies, so it cannot distinguish "absent" from
"present but unreadable". That is the spec's choice and is left alone.

### 3.3 `eprint` / `eprintln`

Two new `Builtin::GLOBAL` members, taking that array from 3 to 5, mirroring `Println`/`Print` at
every seam. They are global rather than `STD_ONLY` because their siblings are.

**CORRECTED 2026-08-11 (Task 5): the line above previously ended "a user function of either name
shadows the builtin, per ADR 0004's shadowing rules" — measured false.** `Builtin::GLOBAL` members
are seeded directly into every module's scope *before* user items are collected
(`crates/nova-resolver/src/lib.rs`'s per-module scope setup), and a user definition of an
already-scoped name is `insert_value`'s ordinary duplicate-definition path, which reports `E0002`
with the note "is a compiler builtin" — not a silent shadow. Measured directly: `nova check` on
`fn eprint(s: String) { }` reports `E0002` ("duplicate definition of `eprint`") with the note
"`eprint` is a compiler builtin", and the same shape for `eprintln`. This is the *same* treatment
`panic` already has (`user_fn_named_panic_is_a_reserved_word`, `nova-resolver`) — `eprint`/
`eprintln` become reserved words the instant they join `GLOBAL`, exactly as `panic` did in Phase
2.1. ADR 0004's shadowing rule is a different mechanism for a different `Builtin` category: it
governs `std/core`'s (and the other std modules') own `pub` items, which are glob-imported into
every module at the *lowest* priority (`or_insert`, so a user definition collected first wins with
no diagnostic) — the opposite priority order from `GLOBAL`/`STD_ONLY`, which are seeded *first* and
block a same-named user definition instead. See `docs/adr/0011-io-error-kinds.md` and the
`CHANGELOG.md` entry for this increment.

### 3.4 The boundary: status first, payload second

Nova has no out-parameters, so an intrinsic returns exactly one word — but `read_to_string` must
convey a `String` *and* a two-field `IoError`. **The status code is the error kind**, so no separate
kind fetch exists:

| Intrinsic | Returns |
|---|---|
| `fs_read_to_string(path)` | `Int` status |
| `fs_take_string()` | `String` — the payload, valid when the status was 0 |
| `fs_read_dir(path)` | `Int` status |
| `fs_take_string_array()` | `[String]` — entry names, sorted |
| `fs_write_string(path, content)` | `Int` status |
| `fs_create_dir(path)` | `Int` status |
| `fs_create_dir_all(path)` | `Int` status |
| `fs_remove_file(path)` | `Int` status |
| `fs_remove_dir_all(path)` | `Int` status |
| `fs_exists(path)` | `Bool` |
| `fs_kind(path)` | `Int` — 0 absent, 1 file, 2 dir |
| `fs_last_error_message()` | `String` |
| `fs_temp_dir()` | `String` — the OS temp-directory path (`std::env::temp_dir()`) |

Twelve new `STD_ONLY` builtins, 17 → 29.

**CORRECTED 2026-08-11 (Task 5): thirteen new `STD_ONLY` builtins, 17 → 30 — not twelve and
17 → 29.** Two things were wrong, not one: the sentence undercounted by exactly the one builtin
the table above was *also* missing, `fs_temp_dir()` (now added as the table's last row). Neither
gap is a rewrite — `fs_temp_dir` is a direct `String` return with no status/payload split, the same
shape `fs_last_error_message()` already has, so it slots into the existing table pattern rather than
needing a new column. Both were stale for the same reason: this section was written before
`fs_temp_dir` moved forward from Task 3 into Task 2 (see
`docs/superpowers/plans/2026-08-11-std-fs-strings.md`'s own Task 2 amendment note), and neither the
table nor this sentence was updated to match once it did. `Builtin::STD_ONLY` is `[Builtin; 30]` as
shipped (`crates/nova-resolver/src/lib.rs`), matching `docs/adr/0011-io-error-kinds.md`.

**No Nova aggregate layout enters Rust.** The runtime never learns that a sum is tag-plus-fields at
`8 + 8i`, nor where `IoError.message` sits. `std/fs`'s Nova wrappers map a status to an
`IoErrorKind` and build every `Result` and `IoError` themselves. This is the whole point of the
split: Nova value layout duplicated in Rust is the one thing this codebase has demonstrably got
wrong before.

**The invariant the thread-local slots rest on, stated so it can be tested rather than assumed:** the
status read, the payload take and the message fetch happen in **straight-line Nova with no `.await`
between them**. The executor is single-threaded and only an `.await` can interleave another task, so
no sibling can clobber a slot mid-sequence. A future `std/fs` function that awaited mid-sequence
would break this, so each wrapper's straight-line shape is load-bearing and carries a comment saying
so.

## 4. Non-goals, each deliberate

- **No byte-based I/O.** `fs::read`, `fs::write`, `open`, `File` all need increment 2's byte type.
- **No `Read`/`Write` traits, no `stdin`/`stdout`/`stderr`.** They need a byte type and either
  references or a settled buffer-returning signature. `impl Trait` in return position does not parse,
  so those constructors would need concrete `Stdin`/`Stdout`/`Stderr` types — which is how `VecIter`
  solved the same problem in 2.2c and needs nothing new, but it is increment 3's decision.
- **No `Formatter`, no `format(parts)`.** String interpolation already works by a different
  mechanism; `Formatter` needs its own design.
- **No poller, no `Wait::Io`, no change to the drive loop.** In particular the drive loop's
  `None => report_deadlock()` default arm is **left alone**: it becomes wrong only when a `Wait`
  variant can carry no deadline, which is increment 4.
- **`exists` stays `Bool`**, per §5, and therefore stays unable to report a permission error.

## 5. Async signatures that never suspend

Every function here is an `async fn` per the binding spec, implemented **synchronously inside the
poll** — no `stage_park`, no suspension, no parking. The signature survives unchanged when a real
poller lands, so no call site breaks later.

**The cost is real and is documented rather than glossed: a `std/fs` call blocks the whole executor
for its duration**, so a sibling task makes no progress during a large read. This is the hazard the
park set's own final review flagged for increment 4. It belongs in ADR form beside ADR 0009's
existing async footguns.

A second consequence: because `block_on` is not callable inside an `async fn`, ordinary use is
`block_on(read_to_string(p))` from a sync `fn main`, or an `.await` inside an `async fn main`.

## 6. Diagnostics

`IoError.message` carries the **OS error text**, because it is often the only thing that explains an
unexpected failure. The consequence is that message text is platform-specific, so any fixture
asserting it must normalise — the same treatment `tests/runtime/nova_test.stdout` already applies to
a Windows NTSTATUS, and for the same reason. Fixtures should assert on `kind` and normalise
`message`.

## 7. Deviations from the binding spec, both needing an ADR

1. **`IoErrorKind` gains `AlreadyExists` and `InvalidData`.** The specced list is network-flavoured:
   two of its six variants are irrelevant to files, while the two most common filesystem failures —
   a binary file handed to `read_to_string`, and `create_dir` on an existing path — would both
   collapse into `Other`. That forces user code to string-match `message` to tell them apart, which
   is precisely what a kind enum exists to prevent. `nova-spec/20-STDLIB.md` §4 is amended to match.
2. **This increment is a subset of §5, not all of it.** `read`, `write`, `open` and `File` are
   deferred to increments 2–3 with the reason recorded.

`ConnectionRefused` is carried unused so §4's network I/O needs no second deviation later.

## 8. Risks

1. **The status↔`IoErrorKind` numbering is a wire contract with two independent copies**, one in
   Rust and one in Nova. This is the same shape as `convert_ty`'s table versus
   `RESERVED_TYPE_NAMES`, and the same shape as the miscompile this project shipped from two lookup
   sites drifting apart. **Mitigation is a round-trip test per kind**, provoking each error for real
   and asserting the resulting `IoErrorKind`, so swapping any two numbers fails. Neither side gets a
   "keep in sync" comment unless that test stands behind it.
2. **`fs_take_string_array` builds a Nova array in the runtime.** That is `str_chars`'s shape one
   step further — an array of `String` pointers rather than `Char`s — and a wrong layout is a silent
   miscompile. It follows `str_chars`'s discipline: assert the tracked `(size, scan)` via
   `gc::object_info`, not merely the values read back.
3. **Two slots, two meanings.** `fs_take_string` reads the payload slot and `fs_last_error_message`
   reads the error slot. Swapping them is a plausible edit that ordinary tests would not catch, so it
   is a named mutation target (§9).
4. **Temp-path collisions in tests.** This repo already carries a latent race from
   `write_test_project` building a run-invariant temp path. Do not copy that pattern; every fixture
   gets a unique path.
5. **`remove_dir_all` may behave differently on Windows than on POSIX** — read-only entries are the
   usual difference. **This is expectation, not measurement: it has not been probed.** Measure it
   during implementation and document whatever it actually does; do not ship the guess above as a
   statement of behaviour.

## 9. Testing

- **One fixture per `IoErrorKind` this increment can produce**, each provoking the condition for
  real: a missing path for `NotFound`, a non-UTF-8 file for `InvalidData`, an existing directory for
  `AlreadyExists`. That set **is** the round-trip test of risk 1.
- **A multi-entry `read_dir` fixture whose entries are created out of alphabetical order**, so
  deleting the sort fails rather than passing by luck.
- **A `std/fs` call inside `block_on` must leave `PARKED` empty** — this pins "never suspends" as a
  property rather than a comment, and it is the test that would catch someone later adding a
  `stage_park` here without the poller work increment 4 requires.
- Assert on `kind` and normalise `message`; **never assert raw OS error text**.
- Every fixture uses a unique temp path and cleans up after itself.
- Suite stays green at 847 + the new tests, with the 8 ADR-0010 tests still ignored and untouched.

Mutation targets, named here rather than left to review:

| Mutation | Must be killed by |
|---|---|
| Swap two status codes | the per-kind round-trip fixtures |
| Drop `read_dir`'s sort | the out-of-order multi-entry fixture |
| Return `0` from a failed operation | that operation's error fixture |
| `fs_take_string` reads the error slot | `read_to_string` on a file whose contents differ from any error text |
| `fs_kind` swaps file and dir | a `read_dir` fixture containing both a file and a subdirectory |
| Add a `stage_park` to a `std/fs` wrapper | the `PARKED`-stays-empty test |

## 10. Definition of done

- All eight functions and `DirEntry` work end to end under both backends, `nova run` and
  `nova build`, and under `NOVA_GC_STRESS=1`.
- `eprint`/`eprintln` write to stderr and are shadowable by a user function of the same name.
  **CORRECTED 2026-08-12 (final-fix wave, review finding I4(a)): "shadowable" is false, and this
  bullet was the surviving copy.** §3.3 above already corrected the identical claim on
  2026-08-11 (Task 5) but this bullet, in the same document, was never updated to match — exactly
  the "sentence this change falsified but did not touch" the quantifier sweep below asks a reader
  to catch, found inside the sweeping document itself. Re-measured (`nova check`): a top-level
  `fn eprint(s: String) { }` or `const eprintln: Int = 1` is `E0002: duplicate definition of
  '<name>'` with the note "is a compiler builtin"; a local `let` or a function parameter, of
  either name, compiles clean in both positions (all four combinations measured) — the clash is
  only checked where `Builtin::GLOBAL` is seeded, at top-level item collection, which neither
  reaches. See §3.3 and
  `docs/adr/0011-io-error-kinds.md`.
- Every `IoErrorKind` this increment can produce is reachable and pinned by a fixture that provokes
  it for real.
- No Nova aggregate layout appears in Rust; the runtime returns only `Int`, `Bool`, `String` and one
  `[String]`.
- `PARKED` is empty after any `std/fs` call, pinned by a test.
- ADR written for both §7 deviations; `nova-spec/20-STDLIB.md` §4 amended.
- Suite green, clippy `-D warnings` and `cargo fmt --all --check` clean.
- **Before committing, run the quantifier sweep** over everything written: grep the added lines for
  `always`, `every`, `only`, `any`, `never`, `all`, `cannot` and, per hit, delete the quantifier or
  state the measurement behind it. Note two things it structurally cannot catch, so check them by
  reading: a doc quoting a literal diagnostic string, and a sentence this change falsified but did
  not touch.
