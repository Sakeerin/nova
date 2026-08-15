# ADR 0011 — `std/io`/`std/fs`'s deviations from `nova-spec/20-STDLIB.md`, and two accepted limitations

**Numbering:** confirmed against `docs/adr/`'s actual contents rather than
trusted from the plan — `0001` through `0010` all exist with no gap, so `0011`
is next. `std/fs/lib.nova` and `std/io/lib.nova` already reference this file by
this exact path (added while Tasks 2–3 were implementing the boundary this
document describes, before this file existed); this document fulfills that
forward reference rather than creating a fresh one.

## Status

Accepted (2026-08-11). Increment 1 of the `std/fs`-on-Strings decomposition
(`docs/superpowers/specs/2026-08-11-std-fs-strings-design.md`), branch
`std-fs-strings`.

## Context

`nova-spec/00-MASTER-SPEC.md` §5's build-order rule and `nova-spec/README.md`
("ADRs (`docs/adr/`) capture decisions that deviate from the spec") both treat
`nova-spec/` as binding: a deviation is only legitimate once it is recorded
here. This increment shipped `std/io`'s error surface (`IoError`,
`IoErrorKind`, `io_error_kind_of`), `eprint`/`eprintln`, and eight `std/fs`
`async fn`s (`read_to_string`, `write_string`, `exists`, `create_dir`,
`create_dir_all`, `remove_file`, `remove_dir_all`, `read_dir`) plus `DirEntry`
and `temp_dir`. Three places in the result differ from `nova-spec/20-STDLIB.md`
§4/§5, and two properties of the shipped functions are limitations accepted
for this increment rather than fixed. All five are recorded below, each with
the reason.

A fourth possible deviation was considered and does not apply: an earlier
draft of the plan worried that building `Result<[DirEntry], IoError>` might
not be reachable from Nova source (no `Vec::to_array`, e.g.) and would force
`read_dir` to return some other shape. Measured during Task 4: `std/fs/lib.nova`
builds `[DirEntry]` directly — a repeat-array literal `[DirEntry { .. }; n]`
followed by indexed assignment — so `read_dir` matches §5's signature exactly,
`Result<[DirEntry], IoError>`, and there is nothing to record here.

## Decision

### 1. `IoErrorKind` gains `AlreadyExists` and `InvalidData`

§4 specifies six variants: `NotFound`, `PermissionDenied`, `ConnectionRefused`,
`TimedOut`, `Interrupted`, `Other`. Two of the six are network-flavoured
(`ConnectionRefused`, `TimedOut`) and irrelevant to a plain file, while two of
the commonest *filesystem* failures have no variant to land on:

- `read_to_string` on a file whose bytes are not valid UTF-8 — Rust's
  `std::fs::read_to_string` reports `ErrorKind::InvalidData` for exactly this,
  and `nova_rt_fs_read_to_string` (`crates/nova-runtime/src/fs.rs`) forwards it
  unchanged.
- `create_dir` on a path that already exists — `ErrorKind::AlreadyExists`.

Both would otherwise collapse into `Other`, and `Other` carries no information
beyond `message` — which is OS-provided, platform-specific text (§6; also
`nova_rt_fs_*`'s `fail` helper, which stashes `e.to_string()` verbatim). A
caller that needs to distinguish "the file isn't text" from "the directory
already exists" would have no choice but to string-match `message`, which is
exactly what a `kind` enum exists to make unnecessary. So `std/io/lib.nova`
ships eight variants:

```nova
pub type IoErrorKind =
    | NotFound
    | PermissionDenied
    | AlreadyExists
    | InvalidData
    | Interrupted
    | TimedOut
    | ConnectionRefused
    | Other
```

`nova-spec/20-STDLIB.md` §4 is amended in place (dated note, this ADR).
`ConnectionRefused` ships unused by `std/fs` — carried forward rather than
dropped, so that `std/io`'s eventual network callers need no second amendment
to add it back.

Both new variants are pinned by a fixture that provokes the real OS condition
and asserts on `kind` — `fs_already_exists.nova` (calling `create_dir` twice)
and `fs_invalid_data.nova` (reading bytes written directly from Rust that are
not valid UTF-8) — the same treatment `fs_not_found.nova` already gives
`NotFound`. **Corrected 2026-08-12:** this paragraph said "three of the eight
variants", which the final review's fix wave falsified by adding a
Windows-gated `PermissionDenied` fixture. **Four** are now pinned by a
real-condition fixture — `NotFound`, `AlreadyExists`, `InvalidData` portably,
and `PermissionDenied` on Windows only, provoked by `read_to_string` on a
directory. (Note the shape matters: reading the temp directory *itself* gives
`NotFound`, because Windows resolves a trailing-backslash-only path before
checking access; the fixture reads a plain created subdirectory.)

**Corrected 2026-08-14 (branch `file-open-openoptions`): "provoked by
`read_to_string` on a directory" is no longer the only way to `PermissionDenied`.**
`open` (decision 2 below) reaches the identical condition a second,
independent way — opening, rather than reading, the same shape of directory —
pinned by `file_open_dir.nova`, `#[cfg(windows)]` for the identical reason.
`NotFound` and `AlreadyExists` are likewise now each pinned twice over, by two
of `open`'s portable failure checks in `file_errors.nova` (a missing parent
directory; `create_new` on a path that already exists). Still **four** kinds
pinned by a real condition, not five — a second path to an already-counted
kind each time, not a new one — and `crates/nova-runtime/src/fs.rs`'s own
status-code doc comment carries the full, corrected per-kind attribution
rather than this paragraph restating it.

The remaining four — `Interrupted`, `TimedOut`, `ConnectionRefused`, `Other`
— are not things `std/fs`'s functions can provoke portably in a test, so
`fs_io_types.nova` instead pins the *numbering* directly — calling
`io_error_kind_of(1)` through
`io_error_kind_of(7)`, the seven codes that map to a specific variant, plus
`io_error_kind_of(0)` (the success status, not itself a kind) and
`io_error_kind_of(99)` (a code nothing defines), both of which must land on
`Other`, and asserting each call maps to the right variant. That
is a weaker guarantee than provoking the OS (it checks the Nova-side mapping
table, not that a real operation ever produces that code), and is stated as
such rather than folded into the same claim as the two real-condition
fixtures. The numbering itself is a wire contract with two independent copies
(`crates/nova-runtime/src/fs.rs`'s status constants and `std/io/lib.nova`'s
`io_error_kind_of`), the same shape that has
already produced a miscompile once in this project, so a fixture per kind is
what actually guards it.

### 2. This increment ships only the `String`/`Bool` subset of §5

§5 also specifies:

```nova
pub async fn read(path: String) -> Result<[u8], IoError>
pub async fn write(path: String, content: [u8]) -> Result<(), IoError>
pub async fn open(path: String, options: OpenOptions) -> Result<File, IoError>
pub record File { /* opaque */ }
impl Read for File { ... }
impl Write for File { ... }
```

All four need a byte type Nova does not have — measured directly (`nova check`
on `fn f(x: u8) -> u8 { x }`): `u8` is not a type, reported as `E0001` ("cannot
find type `u8`") at each of the two positions it appears. `File` and
the `Read`/`Write` traits additionally need either references (`&mut [u8]` is
`E0900`, parsed and rejected in typeck) or a settled buffer-returning shape in
their place, and `stdin`/`stdout`/`stderr`'s declared `-> impl Read` / `-> impl
Write` return types cannot be spelled either way: `impl Trait` in return
position does not parse at all (`P0001`). None of this is a small gap to route
around — it is the reason the whole `std/fs`-on-Strings decomposition exists
as a four-increment plan rather than one. `read`, `write`, `open` and `File`
are deferred to increments 2–3 (a byte type, then concrete `Read`/`Write`/
`File`/`Stdin`/`Stdout`/`Stderr` types), not built here.

What shipped is exactly the eight functions whose signatures need nothing
beyond `String`, `Bool`, `[DirEntry]` and `IoError`: `read_to_string`,
`write_string`, `exists`, `create_dir`, `create_dir_all`, `remove_file`,
`remove_dir_all`, `read_dir`.

**Narrowed 2026-08-12 (branch `byte-type`):** the byte type arrived
(`Bytes`, `docs/superpowers/specs/2026-08-12-byte-type-design.md`), and with
it `read` and `write` — both now exist, over `Result<Bytes, IoError>` and
`content: Bytes`, on three more intrinsics over this same status boundary.
Only `open` and `File` (and, from increment 3's original list above,
`Read`/`Write`/`Stdin`/`Stdout`/`Stderr`) remain deferred. `nova-spec/
20-STDLIB.md` §4 is amended separately (dated note there) to record that
Nova's byte I/O is buffer-**returning**, not the buffer-filling shape §4
originally specified — so references are off the roadmap permanently, not
merely still missing.

**Narrowed 2026-08-14 (branch `read-write-stdio`):** `Read`, `Write` and
concrete `Stdin`/`Stdout`/`Stderr` now exist too (`std/io/lib.nova`), over
new intrinsics on this same status boundary. `Read`/`Write` ship as
`fn ... -> Future<T>`, not `async fn ... -> T`, because the latter is
`E0900` in a trait declaration; `stdin`/`stdout`/`stderr` return the
concrete records `Stdin`/`Stdout`/`Stderr` rather than `impl Read`/`impl
Write`, because `impl Trait` in return position does not parse (`P0001`) —
both narrowings are recorded in `nova-spec/20-STDLIB.md` §4's own dated
note. What remains deferred from this decision's original list is `open`
and `File`, restricted by the property that still separates them rather
than by how many are left: building either needs a handle with real
lifetime management — acquired, held, and eventually closed. `Stdin`,
`Stdout` and `Stderr` needed none of that (process-global, always open,
never closed — `std/io/lib.nova`'s module doc), which is exactly why they,
and the traits over them, could ship ahead of `open`/`File`.

**Closed 2026-08-14 (branch `file-open-openoptions`):** `open` and `File`
were this decision's last remaining item, restricted by the identical
property the note above already named — a handle with real lifetime
management — not by a count. Both now exist (`std/fs/lib.nova`):
`OpenOptions` (six `Bool` flags, `impl Default`, three named constructors —
`reading`, `writing`, `appending`), `File { fd: Int }`, `pub async fn
open(path: String, options: OpenOptions) -> Result<File, IoError>`, an
inherent `pub async fn close(self) -> Result<(), IoError>`, and `impl Read
for File` / `impl Write for File` over the two traits the narrowing above
shipped. This decision's entire original list — `read`, `write`, `open`,
`File`, plus `Read`, `Write`, `Stdin`, `Stdout`, `Stderr` from the narrowing
above — is now shipped: nothing this decision ever named as deferred remains
so, which is what makes this a closing rather than a further narrowing. The
resource model `open`/`File` needed — why a handle survives between calls,
and why closing one is left to the caller rather than the collector — is its
own decision, recorded in `docs/adr/0012-file-descriptor-lifecycle.md` rather
than here.

### 3. `temp_dir() -> String` is added, and is not in §5 at all

A fixture that writes, creates, or removes anything on disk needs a writable
directory and must not hardcode one (a hardcoded path is exactly the
"fixed-name directory" race this project already carries once, in
`write_test_project`, and is not to be copied) — which is most of this
increment's fixtures, though not `fs_not_found.nova`, `fs_io_types.nova` or
`eprint_family.nova`, none of which touch a real path at all. `temp_dir` wraps
`std::env::temp_dir()` and is a **plain `fn`, not
`async`** — unlike its siblings, it only queries the environment and
touches no filesystem, so there is nothing in it that could ever suspend, and
no future reason to change its signature when a poller lands either. §5 does
not list it. `nova-spec/20-STDLIB.md` is not amended for this one: unlike
`IoErrorKind`, §5's function list is a statement of the *eventual* full API,
not a claim that nothing more will ever be added, so there is no existing
sentence for `temp_dir` to contradict — only an omission, recorded here.

## Two accepted limitations

### These `async fn`s never suspend

Every `async fn` `std/fs` exposes runs its filesystem operation
synchronously inside the first poll: there is nothing to suspend on.

**AMENDED 2026-08-16 (branch `io-poller-std-net`): the sentence above used to
read "there is no I/O poller yet, so there is nothing to suspend on", and its
first clause is now stale.** A real poller exists
(`crates/nova-runtime/src/poll.rs`, `docs/adr/0013-io-poller.md`) and
`std/net` suspends on it. The limitation this section records is unchanged,
but its cause is structural rather than temporary: a regular file is not
readiness-pollable by `select`/`WSAPoll` on any of this project's three CI
platforms, so a completion-based interface (IOCP) — declined on scope in
ADR 0013 — is what `std/fs` would actually need. Recorded the same way, and
for the same reason, as `docs/adr/0009-async-execution-model.md` §1's own
2026-08-16 amendment.
`no_filesystem_intrinsic_registers_a_park` (`crates/nova-runtime/src/fs.rs`)
pins this at its source by asserting that `fs.rs`'s own production code
contains no `stage_park` call — the only way a `std/fs` intrinsic could reach
the executor's park set at all, a check that scans everything in the file
before its own `#[cfg(test)] mod tests` block (excluding that block because
the test's own doc comment names `stage_park` in prose) and so covers
whichever `async fn`s `std/fs` exposes without needing a recount as more are
added.

The signature is what matters for forward compatibility: it does not change
when a real poller lands, so no call site written against these functions
breaks then. The cost is real and is recorded rather than glossed: **a
`std/fs` call blocks the whole executor for its duration**, so a sibling task
spawned on the same thread makes no progress while, say, `read_to_string`
reads a large file. This is a new instance of the cooperative-scheduling
hazard `docs/adr/0009-async-execution-model.md` §1 already names under
Consequences — "a long computation inside one task starves every other task on
that thread, and the fix is always an `.await`" — except here the `.await` is
already present at the call site (`read_to_string(path).await`) and does not
help: the callee's poll function runs to completion and reports `POLL_READY`
before the calling task's own suspend machinery is ever reached, so nothing
yields. ADR 0009 §1 is amended in place (dated note, this ADR) to record that
its "always" no longer holds without qualification.

**Narrowed 2026-08-14 (branch `file-open-openoptions`): the guard's reach,
not the property, is what changed.** `File`'s five operations
(`nova_rt_file_open`/`close`/`read`/`write`/`flush`, a new
`crates/nova-runtime/src/file.rs`) are `std/fs` intrinsics too, and by
inspection none calls `stage_park` either, so the never-suspends property
above still holds for them. What no longer holds is
`no_filesystem_intrinsic_registers_a_park`'s own claim, in this section's
opening paragraph, to cover "whichever `async fn`s `std/fs` exposes without
needing a recount as more are added": it scans only `fs.rs`'s own source, and
this is the first increment to put a `std/fs` intrinsic in a second Rust
file, so a `stage_park` call added to `file.rs` alone would not fail that
test.
`File`'s never-suspends property is established here, by inspection and by
`docs/superpowers/specs/2026-08-14-file-open-and-openoptions-design.md`,
rather than by that guard.

### `exists` cannot distinguish "absent" from "present but unreadable"

§5 gives `exists` a `Bool` return, not a `Result`:

```nova
pub async fn exists(path: String) -> Bool
```

so a path that exists but cannot be examined (permission denied on a parent
directory, for instance) is indistinguishable from one that is genuinely
absent — both report `false`. `nova_rt_fs_exists`
(`crates/nova-runtime/src/fs.rs`) calls `std::path::Path::exists`, which has the identical limitation
for the identical reason (it treats any `metadata` error, not only "not
found", as "does not exist"). This is the spec's own choice, not an
implementation shortcut, and is left alone rather than "fixed" into a
`Result` — doing that would itself be an undocumented deviation from §5 in the
other direction.

## Consequences

- **A program written against §4's original six-variant `IoErrorKind` still
  compiles and still matches correctly** — adding variants to a `match`'s
  scrutinee type does not invalidate an already-exhaustive `match` retroactively
  the way *removing* one would; it only means a `match` written after this
  change must additionally cover `AlreadyExists` and `InvalidData` or report
  `E0020`. No existing Nova program predates `IoErrorKind` (it is new in this
  increment), so this is theoretical for today's codebase and stated for
  whoever reads this ADR after a future spec-driven addition.
- **`read`, `write`, `open`, `File`, `Read`, `Write`, `stdin`, `stdout`,
  `stderr` remain entirely unimplemented** — calling any of them is `E0001`,
  identical to before this increment. Nothing here narrows that gap; increment
  2 of the decomposition is where a byte type would have to land first.

  **Narrowed 2026-08-12 (branch `byte-type`):** increment 2 landed the byte
  type and, on top of it, `read` and `write` — both now compile and run.
  `open`, `File`, `Read`, `Write`, `stdin`, `stdout` and `stderr` remain
  `E0001`, exactly as this bullet originally described.

  **Narrowed 2026-08-14 (branch `read-write-stdio`):** `Read`, `Write`,
  `Stdin`, `Stdout` and `Stderr` now compile and run too (`std/io/lib.nova`).
  What remain `E0001` are `open` and `File` — the property that still
  separates them from everything already shipped, rather than a count, is
  that building either needs a handle with real lifetime management, which
  nothing shipped so far has needed.

  **Closed 2026-08-14 (branch `file-open-openoptions`): `open` and `File`
  now compile and run too**, over the identical resource-handle mechanism the
  narrowing above said they were waiting on. Nothing this bullet ever named
  remains `E0001`.
- **The drive loop is untouched.** `crates/nova-runtime/src/task.rs`'s
  `report_deadlock` default arm still assumes every `Wait` variant carries a
  deadline, which is correct as long as nothing can park on an external event —
  true here, since nothing in `std/fs` parks at all. That arm becomes wrong
  only when a real poller adds a `Wait` variant with no deadline (increment 4),
  and is `std/io`'s to revisit then, not this increment's.
- **A future `std/fs` function that awaits mid-sequence would reintroduce the
  hazard the status/payload slots are built to avoid** (thread-local `Cell`
  slots written by one intrinsic and read by the next, with no `.await`
  between them) — noted here because it is the same never-suspends property
  read from the opposite direction: today's straight-line wrappers hold
  because nothing yields mid-sequence, and that is also why they never
  suspend.

  **Corrected 2026-08-13 (branch `per-task-slots`): both halves of this bullet
  are now stale.** The payload storage is no longer thread-local `Cell`
  slots — `crates/nova-runtime/src/fs.rs`'s `Slot`/`Slots`/`SLOTS` replaced
  them with one per-task table, indexed by the current task
  (`docs/superpowers/specs/2026-08-12-per-task-payload-slots-design.md`).
  And the warning itself now points the wrong way: per-task keying is
  specifically what makes a future `std/fs` function awaiting mid-sequence
  *safe*, not hazardous — two tasks stashing between one task's stash and its
  take can no longer collide, because each task's payloads live in its own
  row rather than one slot shared thread-wide
  (`a_stash_is_private_to_the_task_that_made_it`, `fs.rs`). Left standing
  above as the record of the hazard this increment was written to close
  before a poller could make it live.

## Alternatives considered

- **Route `AlreadyExists`/`InvalidData` through `Other` and let callers
  string-match `message`.** This is literally §4 as written, and was rejected
  for the reason decision 1 states: `message` is OS text, is platform-specific
  (§6), and a fixture asserting it must normalise rather than compare exactly —
  building a public API around string-matching it would push that fragility
  onto every caller instead of containing it in one runtime function.
- **Give `std/fs` a minimal `Bytes`-like type scoped to this increment alone**,
  just wide enough for `read`/`write`, rather than waiting for a general byte
  type. Rejected: it would need its own lexer/typeck/MIR/GC-scan support to be
  more than a renamed `[Int]` (which wastes 8 bytes per byte and does not match
  `nova-spec/13-RUNTIME.md`'s FFI story), so it is the same size of work as
  increment 2's real byte type, done in a way this increment could not reuse
  later.
- **Make `exists` return `Result<Bool, IoError>`,** which would close the
  absent-vs-unreadable gap. Rejected as an unrequested, undocumented deviation
  from §5 in the direction opposite to decision 1 — §5 states `Bool`
  deliberately, and the design doc this increment implements already chose to
  leave it alone (`docs/superpowers/specs/2026-08-11-std-fs-strings-design.md`
  §3.2, §4).
- **Give the `std/fs` wrappers a real await point** (e.g. `yield_now().await`
  before returning) purely so a large read does not starve a sibling task.
  Rejected: it would not actually fix the underlying blocking call — the
  filesystem operation itself still runs synchronously to completion before
  that `yield_now` is ever reached — so it would look like a fix while
  changing nothing about the hazard, at the cost of a real behaviour change
  (every `std/fs` call now yields once) that increment 4's real poller would
  have to unwind.

## References

- Spec: `nova-spec/20-STDLIB.md` §4 (`IoErrorKind` amended by this ADR;
  `Read`/`Write`'s buffer parameters amended 2026-08-12 for buffer-returning
  I/O, byte-type design spec below), §5 (`std/fs`, subset shipped; `read`/
  `write` added 2026-08-12, narrowing note above)
- Design: `docs/superpowers/specs/2026-08-11-std-fs-strings-design.md` (§2
  measured facts, §3 the change, §4 non-goals, §5 never-suspends, §7 the
  original two-deviation list this ADR supersedes with three)
- Design (2026-08-12 amendment): `docs/superpowers/specs/2026-08-12-byte-type-design.md`
  — `Bytes`, and the buffer-returning decision that narrows this ADR's §5
  deviation (above) and amends `nova-spec/20-STDLIB.md` §4 (above)
- Design (2026-08-13 amendment):
  `docs/superpowers/specs/2026-08-12-per-task-payload-slots-design.md` — the
  per-task table that replaces the thread-local `Cell` slots this ADR's
  Consequences section originally described (corrected above)
- Design (2026-08-14 amendment):
  `docs/superpowers/specs/2026-08-14-file-open-and-openoptions-design.md` —
  `OpenOptions`, `File`, `open`, `close`, closing decision 2's deviation
  (above) and grounding `docs/adr/0012-file-descriptor-lifecycle.md`
- Plan and ledger: `docs/superpowers/plans/2026-08-11-std-fs-strings.md`,
  `.superpowers/sdd/2026-08-11-std-fs-strings/`
- `crates/nova-runtime/src/fs.rs`: the status constants, `fail`, and
  `no_filesystem_intrinsic_registers_a_park`; `crates/nova-runtime/src/file.rs`
  (2026-08-14): the open-file handle table and `open`/`close`/`read`/`write`/
  `flush`, over the identical status boundary
- `std/io/lib.nova`: `IoError`, `IoErrorKind`, `io_error_kind_of`
- `std/fs/lib.nova`: the wrappers (including `read`/`write` from the
  2026-08-12 amendment above), `DirEntry`, `temp_dir`, and (2026-08-14)
  `OpenOptions`, `File`, `open`
- Related: `docs/adr/0009-async-execution-model.md` §1 (the cooperative-
  scheduling hazard this increment's never-suspends property is a new instance
  of; amended in place by this ADR), `docs/adr/0004-stdlib-compile-model.md`
  (why `std/io`/`std/fs` are Nova source compiled as implicit modules),
  `docs/adr/0012-file-descriptor-lifecycle.md` (2026-08-14 — why `File` has no
  close-on-collect backstop, the decision this ADR's own closed deviation
  needed once `open`/`File` existed to need one)
