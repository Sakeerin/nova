# `Read`/`Write` and the standard streams

**Status:** accepted, not yet implemented. Increment 3b of the `std/fmt` + `std/io` decomposition.
`File`, `open` and `OpenOptions` are **3c**, deliberately split out — see §2.

**Base:** `main` == `origin/main` == `8fb983a`. 891 tests (8 deliberately ignored) across 44 targets;
CI green on ubuntu, macos and windows.

## 1. What this builds

`nova-spec/20-STDLIB.md` §4's `Read` and `Write` traits, and the three standard streams, as
`std/io`'s first executable surface. Today that module holds only `IoErrorKind`, `IoError` and
`io_error_kind_of` — its own module doc names the two things blocking the traits: "`File` and a settled
buffer signature." **The buffer signature is settled** (buffer-returning, `Bytes`, recorded in §4 with a
dated amendment), so `File` was the only remaining blocker — and this increment removes the need to wait
for it by shipping the traits against the streams instead.

## 2. Why the streams and not `File`

`File` would be **Nova's first value holding an OS resource across more than one intrinsic call.**

What was measured, stated as the scan it was: grepping `crates/` for `RawFd`, `RawHandle`, `OwnedFd`,
`OwnedHandle`, `as_raw`, `std::fs::File`, `File::open`, `File::create`, `TcpStream`, `TcpListener`,
`libc::open` and `std::io::std{in,out,err}()` hits exactly two files. One is `nova-driver`, where the
*compiler* reads source files — irrelevant to the runtime. The other is `nova-runtime/src/lib.rs`, where
`nova_rt_print`/`nova_rt_eprint` call `std::io::stdout()`/`stderr()`, **acquire the process-global
stream, write, and drop it inside the one call.** So nothing in the runtime *holds* an OS handle between
intrinsic calls, and increment 3a's per-task slot holds GC memory between two intrinsics, not a handle.

That second hit argues *for* this increment rather than against it: `Stdout` and `Stderr`'s intrinsics
will do exactly what `print` and `eprint` already do — acquire, write, drop — so they inherit a pattern
this runtime has shipped since phase 1, and they hold nothing between calls.

`File` is a different and harder problem: Nova has no destructors, and the GC can collect a record while
the OS handle it names stays open. It needs its own decision about handle lifetime, and `OpenOptions`
needs designing from nothing — **measured: `OpenOptions` appears exactly twice across `nova-spec/` and
`docs/`, in `nova-spec/20-STDLIB.md` §5 and at `docs/adr/0011-io-error-kinds.md:113`, both of which are
`open`'s signature. It is referenced and never defined.**

**Corrected 2026-08-14 (branch `read-write-stdio`, fix round 1): the count above cannot be right, by the
same construction `crates/nova-runtime/src/io.rs`'s `no_stream_intrinsic_can_panic` doc comment already
names for a literal-occurrence claim** — the sentence stating the count is itself an occurrence of the
string it counts, and this document alone names `OpenOptions` in more than one other place (its own
front-matter status line, above `## 1`, and the Non-Goals bullet below), so "exactly twice" was wrong
the day it was written, not only after later edits added more. Restated as a property instead, which
does not go stale the same way: every one of those other mentions only *names* `OpenOptions` as a
still-missing piece of a later increment's scope; the two this paragraph originally cited remain the
only place it appears *inside a signature*; and it is not defined as a type anywhere in `nova-spec/` or
`docs/`, under either kind of mention.

**Corrected 2026-08-15 (branch `file-open-openoptions`, fix round 3; corrected again in fix round 4):
two of the restated property's three clauses are false, and round 3 retracted only one of them.** Round
3 wrote "both halves of that closing clause are now false," which certifies the other two clauses by
omission. It should not have. Clause by clause, each checked rather than inferred:

- **The closing clause** — "not defined as a type anywhere in `nova-spec/` or `docs/`" — is false in
  both halves. The `nova-spec/` half broke at commit `758ad4d`, once `nova-spec/20-STDLIB.md` §5 gained
  an actual `pub record OpenOptions { ... }` declaration. The `docs/` half broke earlier still, and by
  this branch's own first commit:
  `docs/superpowers/specs/2026-08-14-file-open-and-openoptions-design.md` — itself a document under
  `docs/` — has defined `OpenOptions` in full, in its own §2, since commit `1509a82`, whose parent
  `7207a41` is this branch's merge base with `main`. That is three commits after this very correction
  was written (`867b122` → `fc7b252` → `7207a41` → `1509a82`, confirmed by walking the log rather than
  assumed).
- **The middle clause** — "the two this paragraph originally cited remain the only place it appears
  *inside a signature*" — was **already false at that same `1509a82`**, and round 3 did not check it.
  That commit's §2 fence carries four further signature occurrences: `pub fn reading() -> OpenOptions`,
  `writing()`, `appending()`, and `pub async fn open(path: String, options: OpenOptions)`. Fix round 3
  then added three more to `nova-spec/20-STDLIB.md` §5 — those same three constructors.
- **The leading clause** survives only on the narrow reading that scopes "those other mentions" to the
  two elsewhere in this document, the front-matter status line and the Non-Goals bullet, which do still
  name `OpenOptions` as a piece of a later increment's scope. As a present-tense claim over the corpus
  it fails too: the 3c design spec's §2 *defines* it rather than naming it.

None of these falsifications took a misreading to produce — all are the ordinary, expected consequence
of a chartered increment doing exactly the job it was chartered for.

That is the lesson worth keeping, since this paragraph replaced a retracted *count* on the theory that a
property "does not go stale the same way." The lesson is not that a property is more durable than a count
in general. It is narrower and sharper: **a negative existential claimed over an entire directory tree is
falsified by every future commit that adds the thing being denied**, and when the very increment already
scheduled to add that thing is what the document is *about*, the claim was never a stable property to
begin with. It was a date — "as of right now, this is still missing" — wearing a property's grammar, and
it expired the moment the schedule it described was kept.

`Stdin`, `Stdout` and `Stderr` are process-global, always open, and never closed. They exercise both
traits with **no resource-lifetime risk at all**, so `File` arrives in 3c against traits already proven
in use rather than co-designed with them.

## 3. Non-goals

- **`File`, `open`, `OpenOptions`** — 3c. ADR 0011's remaining §5 deviation stays exactly those.

  **Corrected 2026-08-14 (branch `read-write-stdio`, fix round 1): "ADR 0011's remaining §5 deviation
  stays exactly those" is not what that ADR tracks.** `docs/adr/0011-io-error-kinds.md` narrows its
  deviation to `open` and `File` only — `OpenOptions` is not a separate tracked item there, only a type
  referenced inside `open`'s own signature. This increment's own non-goals above are otherwise
  unchanged: nothing here builds `File`, `open`, or the parameter type its signature needs.
- **No I/O poller.** `Wait::Io`, timed waits and the drive loop's default arm remain increment 4's.
- **`print`/`println`/`eprint`/`eprintln` are not touched.** They stay synchronous `Builtin::GLOBAL`
  entries with their existing fixtures. Routing them through an async `Write` would make every
  `println` a future — a behaviour change to shipped functions for no gain. Recorded here so the
  redundancy does not read as an oversight.
- **Not the `Bytes` debt.** `Hash`/`Display`/`Clone`/`Ord` and the `slice`-clamps-versus-panics
  inconsistency stay filed.
- **No `read_line`.** Not in §4, and a line-oriented reader over `Bytes` invites an encoding decision
  this increment does not need.

## 4. The `Future<T>` spelling, and why it is not a workaround

`async fn` in a trait **declaration** is a hard `E0900` ("async methods are not supported yet"), so §4
cannot be written as specified. This spelling works instead, and **every claim in this section about
Nova's own behaviour was measured by running it, on the JIT and in a linked binary.** The one comparison
to Rust below is an analogy for the reader, not a measurement — nothing here rests on it.

```nova
trait Rd {
    fn read(self, max: Int) -> Future<Result<Bytes, IoError>>
    fn read_to_end(self) -> Future<Result<Bytes, IoError>> { read_all(self) }
}
impl Rd for Stdin { fn read(self, max: Int) -> Future<Result<Bytes, IoError>> { do_read(self, max) } }
async fn read_all<T: Rd>(r: T) -> Result<Bytes, IoError> { … r.read(N).await … }
```

Naming `Future<T>` and returning a plain `async fn`'s future **unawaited** is the same shape Rust's
`async fn`-in-trait is generally understood to desugar to — offered as an analogy, unmeasured. What
*is* measured is that Nova accepts it, monomorphizes it, and runs it correctly on both backends, which
is the only thing this design depends on.

Three probes, each run to a printed answer:

| Probe | Result |
|---|---|
| Fieldless `record`, trait impl, `.await` through a method call | prints `2` |
| Trait **default body** delegating to a generic `async fn` bounded by its own trait, which awaits a required method | prints `41` |
| The real nested signature `Future<Result<Bytes, IoError>>` through a defaulted method, both backends | prints `9` and `9` |

The second is what makes `read_to_end` a default rather than a required method: a default body returning
`Future<T>` is **not** `async` and so cannot `await`, but it can *return* the future of a generic
`async fn` bounded by the same trait.

`impl Trait` in return position still does not parse (`P0001`), which is why the streams are three
concrete types rather than `-> impl Read`, exactly as `VecIter` replaced it in phase 2.2c.

**Fixture-writing trap, measured:** a record literal in match-scrutinee position is `P0001` — `match
Stdin { }.read_to_end()` cannot parse. Bind to a local first.

## 5. Surface

```nova
pub trait Read {
    fn read(self, max: Int) -> Future<Result<Bytes, IoError>>
    fn read_to_end(self) -> Future<Result<Bytes, IoError>> { read_all(self) }
}

pub trait Write {
    fn write(self, buf: Bytes) -> Future<Result<Int, IoError>>
    fn flush(self) -> Future<Result<(), IoError>>
}

pub record Stdin { }
pub record Stdout { }
pub record Stderr { }

pub fn stdin() -> Stdin
pub fn stdout() -> Stdout
pub fn stderr() -> Stderr

impl Read for Stdin { … }
impl Write for Stdout { … }
impl Write for Stderr { … }
```

The constructors are **plain `fn`, not `async`** — they allocate nothing and touch no OS state,
following `std/fs::temp_dir`'s precedent for a non-suspending member of an otherwise async module.

`read_all` is a private `async fn` in `std/io`, not part of the surface. It loops `read(CHUNK)` and
concatenates until it sees an empty result. **`CHUNK`'s value is stated once, in `std/io`, and nowhere
else** — a size duplicated between a doc comment and the code is the shape this project has repeatedly
had to correct.

**No type name is reserved and nothing breaks.** `Read`, `Write`, `Stdin`, `Stdout` and `Stderr` are
`std/io` *definitions*, glob-imported into every module, which a user definition of the same name
shadows per ADR 0004. `RESERVED_TYPE_NAMES` stays at **7** — unlike the byte type, this increment adds
no breaking change.

## 6. EOF, and the hazard in it

`read` returns up to `max` bytes. **An empty result means end of stream**, matching the
zero-bytes-read convention.

**A short read is not EOF.** A pipe or a terminal returns what is available, which may be far less than
`max`, and that is normal. So the test for EOF is `len() == 0` and never `len() < max` — getting that
backwards is an infinite loop, not a wrong answer. This must be stated in `Read::read`'s own doc
comment, not only here.

**Corrected 2026-08-14 (branch `read-write-stdio`): "an infinite loop" is wrong, measured directly
rather than reasoned about.** Getting the EOF test backwards does not hang — it produces a wrong
*terminating* answer: early truncation, empty or partial output, exit `0`, instantly. A real pipe or
terminal rarely hands back a chunk exactly `max` bytes long, so `len() < max` reads as "stop" on
nearly the first call, the opposite failure from a loop that never exits. `std/io/lib.nova`'s own doc
comment on `Read::read` and `read_all` already states this correctly; it is what this section's
original claim above should have said.

## 7. The boundary

Five new intrinsics, taking `Builtin::STD_ONLY` from 43 to **48**:

| Intrinsic | Returns |
|---|---|
| `nova_rt_io_stdin_read` | status; payload in the per-task buffer slot |
| `nova_rt_io_stdout_write` | status; byte count in the per-task buffer slot |
| `nova_rt_io_stderr_write` | status; byte count in the per-task buffer slot |
| `nova_rt_io_stdout_flush` | status |
| `nova_rt_io_stderr_flush` | status |

One intrinsic per stream rather than one with a selector, matching `print`/`eprint` already being
separate builtins.

All five use **increment 3a's per-task slot table** from the start. `std/io`'s wrappers collect payloads
with the existing `nova_rt_fs_take_bytes` and `nova_rt_fs_last_error_message`.

**The `fs_` prefix on those two is historical and the spec says so.** Since 3a the slot table has been a
general per-task boundary facility, not a filesystem one; only its name records where it was born.
**Verified that this works without new plumbing:** `nova-resolver/src/lib.rs`'s `resolve_program` seeds
every `Builtin::STD_ONLY` entry into *every* std module's scope via its `if is_std_module(mid)` seeding
loop, not per-module, so `std/io` reaches both takers directly. A note belongs at both definitions and
both call sites. **3c inherits this arrangement**; if a third consumer makes the name intolerable,
renaming is a follow-up with its own review, not a silent widening here.

Each Nova wrapper stays **straight-line with no `.await` between an intrinsic call and its slot read**,
the discipline `std/fs`'s wrappers already follow. 3a made a suspension there survivable across tasks;
it did not make it correct within one.

## 8. The executor hazard, which belongs in ADR 0009 §1

There is no I/O poller until increment 4, so **`stdin` blocks the entire executor.** `read_to_end` on an
interactive terminal blocks until the user sends EOF, which means **a program that spawns tasks and then
reads stdin stalls every other task on the thread indefinitely** — not slowly, but until input arrives.

This is the same class as `std/fs`'s never-suspending `async fn`s, and worse in degree: a filesystem read
finishes on its own, while a terminal read waits on a human. It goes in ADR 0009 §1's footgun list, not
only in a doc comment, because that list is where a reader looks for it.

## 9. Testing

- **A fixture per stream.** Write to stdout and to stderr, and assert both the bytes and which stream
  they landed on — a `Write for Stdout` that wrote to stderr must fail.
- **The `read_to_end` default over a fake reader.** Its loop-and-concatenate behaviour is the one piece
  of real logic in this increment, and the streams cannot exercise it deterministically. A test-only
  `Read` implementor returning known chunks and then empty pins it, including that **two chunks
  concatenate in order** and that an immediately-empty reader yields empty rather than looping.
- **The short-read-is-not-EOF distinction**, via a fake reader whose first result is shorter than `max`
  but non-empty: `read_to_end` must continue, not stop.
- **Error propagation.** A failing intrinsic must surface `IoError` with a `kind`, matching
  `io_error_kind_of`'s existing wire contract with `fs.rs`'s `fail`.
- **`stdin` under `nova build`** as well as `nova run`, per the fs-boundary precedent, with stdin fed
  from the harness rather than a terminal.
- Every wrapper's status check pinned by mutation: deleting it must fail a test. That gap shipped twice
  before — for `write_string`, and again for `fs::read`/`fs::write`.

## 10. Risks

| Risk | Mitigation |
|---|---|
| A wrapper `.await`s between an intrinsic call and its slot read | Straight-line wrappers; the discipline is stated in `std/io`'s module doc as it is in `std/fs`'s |
| `read_to_end` loops forever on a short read | §6's `len() == 0` rule, pinned by the short-read fixture |
| A payload slot read on a failure path returns a stale buffer | Status checked before any take, as `std/fs` does |
| The `fs_`-prefixed takers mislead a future reader | Notes at both definitions and both call sites; renaming deferred to a follow-up with review |
| `stdin` blocking the executor surprises someone | ADR 0009 §1 entry, plus `Stdin`'s own doc |

## 11. Definition of done

- `Read` and `Write` in `std/io` with the `Future<T>` spelling; `read_to_end` a default over `read`.
- `Stdin`/`Stdout`/`Stderr` with plain-`fn` constructors and the three impls.
- Five intrinsics over 3a's per-task slots; `STD_ONLY` at 48; `STD_MODULES` unchanged at **7**;
  `RESERVED_TYPE_NAMES` unchanged at **7**.
- `nova-spec/20-STDLIB.md` §4 amended in place with a dated note recording the `Future<T>` spelling and
  that `impl Read`/`impl Write` return types became concrete types — original text preserved.
- ADR 0011's deviation narrowed to `open`/`File` only in wording that reflects the traits now existing.
- ADR 0009 §1 gains the stdin-blocks-the-executor entry.
- `std/io`'s module doc no longer says the traits "arrive in a later increment".
- `cargo build --workspace` before `cargo test --workspace --no-fail-fast`; suite green with the count
  risen by exactly the tests added; clippy `--all-targets --all-features -- -D warnings` and
  `cargo fmt --all --check` clean; the 8 ADR-0010 ignored tests still ignored and untouched.
- `CHANGELOG.md` under `### Added`. **Nothing belongs under `### Changed`** — no existing behaviour
  moves and no name is reserved — but **verify that against the heading's own stated scope rather than
  assuming it.**
