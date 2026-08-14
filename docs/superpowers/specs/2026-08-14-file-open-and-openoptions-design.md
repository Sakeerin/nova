# `File`, `open` and `OpenOptions`

**Status:** accepted, not yet implemented. Increment 3c of the `std/fmt` + `std/io` decomposition, and
the last of it — increment 4 is the I/O poller.

**Base:** `main` == `origin/main` == `7207a41`. 907 tests (8 deliberately ignored) across 44 targets;
CI green on ubuntu, macos and windows. `STD_ONLY` at 48; `STD_MODULES` and `RESERVED_TYPE_NAMES` both
at 7.

## 1. What this builds, and why it is the hard one

`nova-spec/20-STDLIB.md` §5's `open` and `File`, plus the `OpenOptions` its signature references and
which **no document defines as a type**. The normative documents — `nova-spec/20-STDLIB.md` and ADR 0011 —
name it only as `open`'s parameter type; everywhere else it appears, it is prose *about* that gap. **This
spec is the first document to define it**, which is stated as a property rather than a count of
occurrences: the 3b spec asserted "exactly twice" for this same identifier, and that was false the day it
was written, because the sentence making such a claim is itself an occurrence and the grep behind it had
filtered out part of the tree it claimed to cover.

ADR 0011's remaining deviation is exactly `open` and `File`, restricted by the property that separates them
from everything already shipped: **they need real handle lifetime management.**

`File` is **Nova's first value holding an OS resource across more than one intrinsic call.** Everything
shipped so far avoids the problem rather than solving it. `std/fs`'s existing functions are one-shot —
they open, act and close inside a single intrinsic. Increment 3b's five stream intrinsics deliberately
inherit that same acquire-act-drop shape, which is precisely why `Stdin`/`Stdout`/`Stderr` needed no
lifetime management at all: the process streams are always open and never closed.

**Nova has no destructors and no finalizers**, and the GC can collect a record while an OS handle it
names stays open. So this increment's real content is §3, not §2.

## 2. Surface

```nova
pub record OpenOptions {
    read: Bool, write: Bool, append: Bool,
    truncate: Bool, create: Bool, create_new: Bool
}

impl Default for OpenOptions { /* every field false */ }

impl OpenOptions {
    pub fn reading() -> OpenOptions      // read
    pub fn writing() -> OpenOptions      // write + create + truncate
    pub fn appending() -> OpenOptions    // append + create
}

pub record File { fd: Int }

pub async fn open(path: String, options: OpenOptions) -> Result<File, IoError>

impl File {
    pub async fn close(self) -> Result<(), IoError>
}

impl Read for File  { fn read(self, max: Int) -> Future<Result<Bytes, IoError>> }
impl Write for File {
    fn write(self, buf: Bytes) -> Future<Result<Int, IoError>>
    fn flush(self) -> Future<Result<(), IoError>>
}
```

**`close` is inherent, so it uses plain `async fn`.** The `Future<T>` spelling increment 3b introduced is
forced only on *trait* methods, because `async fn` in a trait declaration is `E0900`. It is `async` for
the reason every `std/fs` function is: the signature must not change when a poller lands.

**All six flags ship, not a YAGNI subset.** They cost nothing at the boundary — the intrinsic forwards
them to `std::fs::OpenOptions` — and trimming them would need a second increment to add the ones a real
program wants.

**Three named constructors, and no builder.** Measured: a receiver-mutating method **cannot be called on
a temporary** (`E0060`), so `OpenOptions::reading().with_write()` does not compile and a chainable builder
is not available in this language. Exotic combinations use `let mut o = OpenOptions::default()` followed by
field assignment; `impl Default`, associated constructors, and field assignment on a `let mut` local were
each verified by running them.

**No type name is reserved and nothing breaks.** `File` and `OpenOptions` are `std/fs` *definitions*,
glob-imported into every module, which a user definition of the same name shadows per ADR 0004.
**`RESERVED_TYPE_NAMES` stays at 7** — unlike the byte type, this increment adds no breaking change. If an
implementation finds itself editing that constant, the design has been misunderstood.

## 3. The resource model

`fd` is a key into a thread-local table of open `std::fs::File`s, living in a new
`crates/nova-runtime/src/file.rs`. Thread-local for the reason `task.rs`'s module doc gives for `TASKS`
and `QUEUE`, and `fs.rs` for its slot table: the GC's roots are per-thread, so a second thread running
Nova code would free objects the first still holds.

**Explicit `close` is the only release mechanism. Forgetting it leaks the descriptor until the process
exits.** That is the same shape ADR 0009 §1 already documents for a spawned task whose output is never
taken — inherited, not invented.

### Close-on-collect is deliberately foreclosed, and this is the decision to record

The collector **already has a per-object notification hook.** `gc.rs`'s sweep calls
`crate::task::forget_freed_state(o.addr)` at the moment an address stops being live, and that call site
documents the constraint such a hook must satisfy: it is called with `HEAP` borrowed, so it must touch
nothing the collector owns, allocate nothing through `alloc`, and not re-enter the collector. Closing a
file is a syscall with no allocation, so it fits that shape. **A reader who finds that hook will
reasonably ask why `File` does not use it.**

Two measured reasons:

1. **`fd: Int` makes it impossible, not merely unused.** The collector sees GC objects. It never sees an
   integer inside a record, so it cannot know a `File` died. Keeping the option open would require `File`
   to hold a GC-managed cookie whose address keys the table — `JoinHandle { fut: Future<T> }`'s
   pointer-identity pattern, which a prior increment deliberately migrated *toward*. That was considered
   and declined for simplicity; §6 records it.
2. **Collection does not run at all off Windows.** `gc.rs`'s `stack_base()` returns `None` for
   `#[cfg(not(windows))]`, with the comment "until then collection is skipped there." So a close-on-collect
   backstop would clean up on Windows and leak unboundedly on Linux and macOS. **A platform asymmetry in
   resource correctness is worse to document than a uniform leak**, and it would encode a guarantee two of
   three supported platforms cannot honour.

**Consequence to state plainly rather than bury:** a long-running program that opens files in a loop and
forgets to close them will exhaust file descriptors, on every platform, and no collection will save it.
That is the accepted cost of this increment's simplicity.

### Idempotent close, and use-after-close

**`close` is idempotent**: a second call is a no-op returning `Ok`. **Any other operation on a closed
`File` returns `IoError { kind: Other }`** whose message names the condition.

This follows the precedent `task_release` and `JoinHandle::join` set for exactly this situation, and their
own doc comments give the reason: **Nova has no move checking, so `self` by value cannot stop a second
call.** `close(f)` followed by `f.read(…)` compiles cleanly, which makes use-after-close not an exotic
mistake but the natural consequence of the language. Aborting on it — the treatment `bytes_at`'s
negative-index guard and `bytes_from_ints`'s range guard get — would kill a process for a mistake the
language positively invites.

`Other` rather than a new `IoErrorKind::Closed`: `IoErrorKind` is public, and its mapping is **one half of
a wire contract** with `fs.rs`'s `fail` — two independent copies of one numbering, which this project's own
documentation calls the shape that has already produced a miscompile. A kind no filesystem condition
produces is not worth widening that contract for.

## 4. The boundary

New intrinsics for open, close, read, write and flush, taking `STD_ONLY` past 48. Payloads travel on the
**per-task slot table** `fs.rs` has owned since increment 3a, exactly as `std/io`'s do; `std/fs`'s new
wrappers stay **straight-line, with no `.await` between an intrinsic call and its slot read**. The
per-task table makes a suspension there survivable across tasks; it does not make it correct within one,
because the task's own next call overwrites its own slot.

**Adding one intrinsic touches ten seams across four crates**, and the tenth — `symbols()` in
`nova-runtime/src/lib.rs` — is keyed by the lowercase symbol string and invisible to a PascalCase grep. A
missing entry compiles, links, and fails only when compiled code actually calls it; the `nova_rt_task_*`
entries shipped in exactly that state once, which is why `every_rt_func_symbol_is_registered_with_the_jit`
exists. The implementation plan must carry that seam list.

**No panic may cross a generated poll boundary.** `abort_with` is acceptable; `unwrap`, `expect`, `RefCell`
borrow panics, slice indexing and fallible `format!` are not. The table access is the new hazard, and it
takes the same treatment `fs.rs`'s slot table got: a fallible borrow with an `abort_with` backstop.

## 5. Testing

What only this increment can get wrong is the resource model, so that is where the tests go.

- **A round-trip through a real file:** `open` for writing, `write`, `close`, `open` for reading,
  `read_to_end`, `close`, and assert the bytes match. Exercises both trait impls and the `Read`
  default inherited from increment 3b.
- **Use-after-close returns an error** rather than aborting, reading freed memory, or succeeding. This is
  the single most important test here: it is the one behaviour the language cannot enforce.
- **Double-close is a no-op returning `Ok`**, and a third call likewise.
- **Two `File`s open at once do not collide** — write distinct content through each, close both, and
  assert each file has its own bytes. A table keyed by a counter that failed to increment would pass every
  single-file test.
- **Every wrapper's status check pinned by mutation.** Unlike the standard streams, failure here is
  reachable portably: a directory path where a file is expected, a missing parent directory, and
  `create_new` on an existing path all fail on every platform. **Deleting any wrapper's status check must
  fail a test** — that gap shipped twice before, for `write_string` and again for `fs::read`/`fs::write`.
- **`OpenOptions`'s flags reach the OS**: `create_new` on an existing path is `AlreadyExists`; `reading()`
  then attempting a write fails. Options that are silently ignored are the plausible partial
  implementation.

The leak-on-forget path is **not** tested. Asserting that a descriptor stays open is neither portable nor
meaningful, and a test that merely opens without closing proves nothing. §3 documents it instead.

## 6. Alternatives considered

**A GC-managed cookie in `File`, table keyed on its address.** Matches `JoinHandle`'s pointer identity and
would keep close-on-collect available the day non-Windows stack bounds land. Declined for simplicity: it
costs an allocation per `open`, and Nova has no opaque-cookie type, so the only available carrier is
`Bytes` — a dishonest field type for something that is not byte data. **This is the alternative to revisit
first** if descriptor leaks turn out to bite in practice.

**A first-class opaque `Ty` variant for `File`**, the route `Bytes` took. The most honest representation,
and the collector could see it. Declined on cost — roughly ten compiler seams — and because it would
**reserve the name `File`**, a breaking change this increment otherwise does not have.

**A scope-based `with_file(path, options, body)`** that closes on the way out, making the common case
leak-proof by construction with no destructor. Genuinely safer, and worth building later. Declined here
because it is surface the spec never asked for, error threading through a closure needs its own design, and
it would widen an increment whose job is settling the resource model.

**A Windows-only close-on-collect backstop.** Declined: see §3.

## 7. Also in scope

Two clauses increment 3b left for this increment, both one sentence:

- **`nova-spec/20-STDLIB.md` §4 never states that a short write is legal**, though `-> Result<Int, IoError>`
  implies a count. `Write::write` is plain `write`, not `write_all`.
- **`nova_rt_io_stdin_read` allocates the caller's `max` eagerly**, so a generous *ceiling* is charged in
  full: `stdin().read(1000000000000)` aborts on allocation before the `Err` arm runs. Not a regression
  (`"Q".repeat(huge)` aborts identically on `main`) and not a poll-boundary breach (`handle_alloc_error`
  aborts rather than unwinding), but `read(max)` expresses a ceiling where `repeat(n)` asks for n, and
  neither doc says so.

## 8. Definition of done

- `OpenOptions`, `File`, `open`, `close`, `impl Read for File`, `impl Write for File` in `std/fs`.
- A thread-local handle table in `crates/nova-runtime/src/file.rs`, fallible-borrow with an `abort_with`
  backstop, over the per-task slot boundary.
- `close` idempotent; use-after-close an `IoError { kind: Other }`, both pinned by tests.
- `STD_ONLY` updated to its measured new value; **`STD_MODULES` and `RESERVED_TYPE_NAMES` unchanged at 7**.
- `nova-spec/20-STDLIB.md` §5 amended in place with a dated note defining `OpenOptions` and recording the
  handle-lifetime decision — **original text preserved**.
- **ADR 0011's deviation closed**, since `open` and `File` were its last two items.
- A new ADR, or an ADR 0009 amendment, recording that **close-on-collect is foreclosed by the `Int`
  representation** and why — so the next person who finds the sweep hook does not read its absence as an
  oversight.
- `cargo build --workspace` before `cargo test --workspace --no-fail-fast`; suite green with the count
  risen by exactly the tests added; clippy `--all-targets --all-features -- -D warnings` and
  `cargo fmt --all --check` clean; the 8 ADR-0010 ignored tests still ignored and untouched.
- `CHANGELOG.md` under `### Added`. Nothing should belong under `### Changed` — no name is reserved and no
  existing behaviour moves — but **verify that against the heading's own stated scope rather than assuming
  it.**
