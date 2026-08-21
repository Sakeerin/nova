# ADR 0017 — `std/sync`'s bounded channel: the shape §13 could not express

**Numbering:** confirmed against `docs/adr/`'s actual contents rather than
trusted from the plan — `0001` through `0016` all exist with no gap, so
`0017` is next. A previous increment guessed a number already in use; this
one listed the directory.

## Status

Accepted (2026-08-21). The `std/sync` bounded-channel increment, branch
`std-sync-channel`
(`docs/superpowers/specs/2026-08-21-std-sync-channel-design.md`).

## Context

`nova-spec/20-STDLIB.md` §13 specifies a bounded channel for `std/sync`,
and it specifies it in a form Nova cannot compile:

```nova
pub record Channel<T> { /* opaque */ }
pub fn channel<T>(buffer: Int) -> (Sender<T>, Receiver<T>)
```

The return type is a tuple, and Nova's type system has none — measured,
`error[E0900]: tuple types are not supported yet`, whose own note says the
feature "arrives in a later milestone". `docs/adr/0016-std-sync-partial-close.md`
recorded that blocker at its own date, at three separate sites — its
Decision headline (`:60`), its deferral bullet (`:154-155`) and its
Consequences (`:195`) — and named the remedy without taking it: the
channel "needs an API redesigned around what Nova can express — a record
holding both ends, or two separate
constructors — which is a design decision, not a transcription, and
belongs to its own increment." This is that increment.

Three of §13's own oddities shaped the redesign rather than a free choice
of API:

- §13 **declares** `pub record Channel<T>` on the line immediately above
  the function and then never returns it, never gives it an `impl`, and
  never mentions it again. It also never declares `Sender<T>` or
  `Receiver<T>` at all, though the signature names both.
- The two named-but-undeclared types are the pair the tuple was carrying,
  so any shape that keeps the producer/consumer split has to invent their
  declarations regardless.
- `T` occurs only in the return type, so no argument can infer it, and
  Nova has no turbofish.

## Decision

### 1. The signature deviates, and it deviates *toward* a spec type

Shipped, in `std/sync/lib.nova`:

```nova
pub record Channel<T> { ring: Ring<T>, cap: Int, closed: Bool }
pub record Sender<T> { ch: Channel<T> }
pub record Receiver<T> { ch: Channel<T> }

pub fn channel<T>(buffer: Int) -> Channel<T>

impl<T> Channel<T> {
    pub fn sender(self) -> Sender<T>
    pub fn receiver(self) -> Receiver<T>
}
```

`channel` returns `Channel<T>` — the record §13 declares and never uses —
and the pair is reached through `ch.sender()` and `ch.receiver()`. All
three of §13's type names are now built, and `Ring<T>` (private) is the
only name added that §13 does not have. **The deviation is confined to the
return type, and the type it moved to is one the spec already declares**,
which is why this is recorded as a deviation from §13's signature and not
as a new API §13 does not describe.

`Sender` and `Receiver` are two views onto one channel. Records are
reference types in Nova, so both observe every mutation; the split carries
intent — hand `tx` to producers and `rx` to consumers — and not
enforcement.

Semantics, all measured by fixture:

- `try_send` returns `false` when the channel is **full or closed**; the
  async `send` returns `false` **only** when closed, waiting out a full
  channel. That asymmetry is why `Channel::push` reports one `false` for
  two conditions and `send` re-reads `closed` itself to tell them apart.
- `try_recv` returns `None` when **empty**; the async `recv` returns `None`
  **only when closed and drained**, because every iteration tries `pop`
  before reading `closed`. Buffered values therefore drain before a close
  is reported.
- `close` is idempotent, and buffered values stay readable after it, so a
  producer may close the instant it stops producing.
- `buffer < 1` **clamps to 1** rather than panicking. A zero-capacity
  rendezvous channel needs a handoff protocol yield-and-retry cannot
  express without a second wait state; clamping follows `Int::pad`'s
  precedent of an early return over a panic
  (`std/fmt/lib.nova:27`, `if len >= width { return s }`).

### 2. Reverting to the tuple when tuples land is optional, not owed

A later reader who finds `error[E0900]` gone should not read this ADR as a
promise to "restore" the tuple. When tuples land, the accessor form
remains defensible on its own merits: `Channel<T>` gives the pair an
**identity**, and an identity can grow accessors — a second receiver, a
capacity query, a closed query — without changing `channel`'s signature or
breaking any call site. A tuple return cannot; every addition to it is a
breaking change to arity. **Restoring the tuple would be a fresh design
decision requiring its own justification, not the completion of a deferred
one.** This paragraph exists because the alternative reading is the
available one and nothing else in the tree forecloses it.

### 3. Every call site must annotate, and both escapes were refused

```nova
let ch: Channel<Int> = channel(2)      // the only available form
let ch = channel(2)                    // no way to name T
```

`T` appears only in the return type, so nothing infers it from the
argument, and Nova has no turbofish: `channel<Int>(2)` is not a call but a
parse error, read as two chained comparisons —
`error[P0001]: chained comparison operators are not allowed (use parentheses)`.
Every fixture and every doc example therefore shows the annotated form.

Two escapes were considered and both refused:

- **A seed parameter** (`channel(2, 0)`) would make `T` inferable *and*
  delete the lazy-allocation branch in `push` outright — genuinely two
  wins. Refused because it puts a parameter in the public signature that
  exists only because `[fill; n]` needs a value of `T`: an implementation
  detail promoted to API, permanently, to save an annotation.
- **`Channel::new(buffer)`** as an associated function has the *identical*
  inference problem — `T` still occurs only in the return type — so it
  would need the same annotation while moving further from §13's `channel`
  free function. It is not an escape at all.

The annotation is a real ergonomic cost and is recorded as one, not
argued away. Rust's own `mpsc` needs the same annotation when the element
type is otherwise unconstrained.

### 4. Yield-and-retry again, for the reason ADR 0016 gave

Both async methods wait by `yield_now().await` in a retry loop, not by
parking:

```nova
while !self.ch.push(v) { if self.ch.closed { return false } yield_now().await }
```

This keeps the whole module in Nova with **zero new intrinsics**
(`Builtin::STD_ONLY` stays at 65) and **no executor change**: no fourth
variant beside `Wait::Deadline`/`Task`/`Io`
(`crates/nova-runtime/src/task.rs:162`) and no arm added to any of the
three `parked.retain(...)` matches (`task.rs:771`, `:1097`, `:1153`),
each of which ends in a wildcard rather than an exhaustive list — where an
omitted arm parks a task forever with no compile error and no diagnostic.
ADR 0016 declined parking for `Mutex`; declining it again here is
consistency, not inertia.

**The cost, stated rather than hidden:** a waiter stays *runnable* the
whole time it waits. `report_deadlock` (`task.rs:1243`) is reached from
exactly one place, `task.rs:1007`, and only when the ready queue is empty
and the park set holds nothing that can refill it. A spinning task keeps
the ready queue non-empty, so that arm is unreachable while any task
spins: a channel nobody drains does not deadlock visibly, it spins
forever instead. No fairness guarantee is made either — which waiter
proceeds next, when several are spinning, is unspecified.

`close` being explicit follows from the same absence ADR 0016 recorded for
`MutexGuard::release`: `Drop` appears in three spec files
(`12-TYPESYSTEM.md:192`, `13-RUNTIME.md:96`, `14-CODEGEN.md:24`) and is
implemented in none, so nothing closes a channel on a producer's behalf.
Without an explicit `close`, `recv`'s "closed and drained" answer would
have no source and a consumer loop could not terminate.

### 5. Two hazards are documented, not prevented

Both are the shape ADR 0012 records for `File { fd: 9999 }` and ADR 0016
for a forged `MutexGuard`, and both follow from one fact: **Nova enforces
no field privacy.**

- **Forgeable handles.** `Sender { ch: c }` is ordinary legal code, so the
  producer/consumer split is intent and not enforcement — a forged
  `Sender` can send or close on a channel it was never given. It needs a
  `Channel<T>` to name, which is the same limit the forged-guard hazard
  has.
- **Invisible spin.** A waiter is runnable, so §4's `report_deadlock`
  blind spot is reachable from ordinary correct-looking code: forget to
  drain, and the process neither aborts nor progresses.

### 6. ADR 0016's blocker is superseded here by cross-reference, not edited

ADR 0016 states that the channel is deferred and blocked on tuple types
at **three** sites, not two: its Decision headline (`:60`, "defer
`channel<T>`"), its deferral bullet (`:154-155`) and its Consequences
(`:195`). The set was enumerated by grepping every `channel` mention in
that file rather than taken from a hand-off, which is how the headline
turned up — it is the most prominent of the three and the easiest to
supersede by accident. A fourth mention, `:17`, describes what §13
*specifies* rather than what was built, and stays true: §13's code block
still literally writes the tuple signature. **Those lines are correct as
of ADR 0016's date (2026-08-20) and are deliberately left untouched.** A
dated ADR records
what was true when it was written; editing it would destroy the record of
the decision rather than update it. The blocker is resolved **by
redesign** — Nova still has no tuples, and `error[E0900]` is still exactly
what §13's literal signature produces. What changed is the signature, not
the type system. Anyone reading ADR 0016's deferral list should read this
ADR beside it.

## Consequences

### Position 8 is *still* partial, and three documents disagree about why

**This increment does not close position 8, and no record of it should say
it does.** The three documents that describe `std/sync`'s contents do not
agree on what those contents are:

| Source | Names |
|---|---|
| `nova-spec/20-STDLIB.md:27` (module index) | `Mutex, RwLock, channel, atomic` — **four** |
| `nova-spec/00-MASTER-SPEC.md:238` (§3 build order, position 8) | `Mutex, channel, atomic` — **three**, omitting `RwLock` |
| `nova-spec/20-STDLIB.md` §13 (the specification) | `Mutex` and `Channel` — **two**, with signatures |

The honest claim, and the only one this ADR makes: **`channel` completes
§13's own specification of `std/sync`.** Both types §13 gives signatures
for are now built. `RwLock` and `atomic` remain named **only in index
lines that no section specifies** — neither has a signature, a semantic,
or a section anywhere in `nova-spec/`, so neither can be built from the
spec as it stands and neither is merely un-got-to. Against the
four-item index, two of four ship; against the three-item build order, two
of three; against §13, two of two.

The distinction matters because ADR 0016's own Consequences section mixed
position-7 items (`spawn_blocking`, `JoinHandle::cancel`, both `std/task`)
into the same discussion as position-8 items, and a later reader
summarising loosely can produce a denominator that belongs to neither
position. Position 7 is not in scope here at all: `spawn_blocking` still
needs a thread pool in a runtime whose own first line calls itself "A
single-threaded cooperative executor", and `JoinHandle::cancel` still
needs an interrupt hook the poll ABI does not have. Neither is touched by
this increment and neither moved.

### A second spec file contradicts the tree, and it is out of scope here

`nova-spec/13-RUNTIME.md` §4.5 (`:129-136`) specifies the channel a third
way again:

```nova
let (tx, rx) = sync.channel::<Int>(buffer: 100)
```

Four separate impossibilities in two lines — tuple destructuring, a
turbofish, a named argument, and `sync.` as a module path — plus "Backed
by Tokio's `mpsc::channel`", where **the workspace depends on no Tokio at
all** (`grep -i tokio` over every `Cargo.toml` returns nothing; the
runtime is a hand-written single-threaded executor). This is not a
contradiction this increment introduced and not one it fixes. It is
recorded here because it is the obvious next thing a reader checking §13
will find, and because the reason for leaving it is structural rather than
convenient: **every dated amendment under `nova-spec/` lives in
`20-STDLIB.md`, and every other spec file in that directory carries
none** — including `13-RUNTIME.md`, which already describes Tokio on
seven separate lines and specifies structured cancellation that does not
exist. **No total is given here on purpose.** This increment adds an
`AMENDED` marker to `20-STDLIB.md`, so any count of them stated in this
ADR would be falsified by the very commit that states it — a draft of
this paragraph said "all 15", measured correctly before the amendment was
written and wrong by one after. The property survives the next amendment;
an arithmetic claim about a population this commit changes cannot.
Amending `13-RUNTIME.md` here would invent
a convention for a file whose staleness is systemic and much larger than
`std/sync`. It wants its own increment, and this ADR names it so the next
reader does not have to rediscover it.

### What is not testable, and why that is load-bearing

The eight `channel_*` fixtures pin every semantic above except one, and
the exception is not an oversight — it is unreachable by construction, so
recording it is the only available mitigation.

**The livelock cannot be asserted directly.** "The retry loop suspends"
can only be tested by driving the code into the retry loop with the
suspension removed, and that state is a *livelock*, not a wrong answer: a
task spinning with no suspension point is unpreemptable on a
single-threaded cooperative executor, so the executor never regains
control, no watchdog can fire, and nothing observes anything. Any fixture
asserting it directly **must hang rather than fail.** There is no bounded
version of that test.

What is bounded is the mutation's **first** consequence rather than its
eventual one. Moving a yield to the front of the method makes it fire even
when no wait is needed, so:

> A `recv` (or `send`) that can answer immediately must answer **without**
> yielding.

That is terminating and order-observable. Two fixtures pin it —
`channel_recv_suspends_only_to_retry` and
`channel_send_suspends_only_to_retry` — by using a second task that only
prints, turning the suspension into output order. Both mutations
previously left the **entire 44-target suite green**, and now each fails
exactly one test as an ordinary stdout diff in **~12s** (12.5s and 11.0s
measured), not a timeout. Neither fixture pins the retry path alone, and
their comments say so: the retry path is pinned *jointly* with
`channel_two_tasks_blocking`, which pins that `recv` waits **at all**, and
a wait that exists but is not on entry is a wait in the retry path.

**One mutation still manifests as a hang, and the accurate description of
it matters.** Making `recv` return `None` on an empty-but-open channel was
predicted to make output "truncate". Measured: it truncates — to
`1 2 done` — **and then hangs forever.** The consumer exits early, so
nothing drains again, so the producer's `send` spins on a full channel
while staying runnable, and §4's blind spot is exactly why nothing
notices. It cost a ten-minute suite timeout and left an orphaned
`nova.exe` to kill. The prediction was not merely incomplete; the missing
half is the expensive half, and any future mutation testing on this module
wants a per-test cap or a `--skip`. That this mutation reproduces the
documented cost of yield-and-retry, unprompted, is the strongest available
evidence that the cost is real and not a formality.

**The real fix is not a fixture.** It is a `report_deadlock` that can see
runnable-but-starving tasks rather than only parked ones — which ADR 0016
already files as accepted-not-fixed. Nothing in a test can substitute for
it, and this ADR does not claim otherwise.

### Everything else that did not move

- **Zero intrinsics.** `Builtin::STD_ONLY` stays at 65,
  `RESERVED_TYPE_NAMES` at 7 — `Channel`/`Sender`/`Receiver` are ordinary,
  glob-imported, shadowable `std/sync` records, the standing `Mutex`,
  `TcpStream` and `File` already have.
- **`STD_MODULES` stays at 12.** `std/sync` joined the array last
  increment; this one only grows its `lib.nova` (166 → 307 lines).
- **The poll ABI, the executor, and `nova-runtime` are untouched.** The
  only non-Nova change in the entire branch is fixture registration in
  `crates/nova-cli/tests/run_tests.rs`.
- **Suite: 44 targets / 1044 passed / 0 failed / 8 ignored.** The eight
  ignored are ADR-0010's GC tests, untouched.

## References

- Design: `docs/superpowers/specs/2026-08-21-std-sync-channel-design.md`;
  plan `docs/superpowers/plans/2026-08-21-std-sync-channel.md`
- `docs/adr/0016-std-sync-partial-close.md`: the immediate predecessor.
  Its tuple blocker — stated at `:60`, `:154-155` and `:195`, an
  enumerated set and not a partial one — is **superseded by this ADR's
  Decision 6 and deliberately not edited**; it was true at its date. Its
  yield-and-retry decision and its `report_deadlock` blind spot
  (`:205-231`) are followed here rather than re-derived, and the blind
  spot is still filed there as accepted-not-fixed
- `docs/adr/0012-file-descriptor-lifecycle.md`: the precedent for
  documenting a forgeable handle rather than preventing it, and for
  explicit release over a collector backstop. Cited for the *shape* of the
  response only — its argument about the collector's per-object hook
  reporting a freed address is specific to a runtime-managed handle and
  does not transfer to a channel, which never reaches the collector
- `docs/adr/0009-async-execution-model.md`: the single-threaded
  cooperative executor these two async methods live on; `spawn_blocking`
  recorded as un-honourable (`:107`) and cancellation as an **open
  residual gap** (`:365`) — both position-7 items, neither touched here
- `docs/adr/0010-conservative-scan-root-test-gating.md`: the 8 ignored
  tests, unchanged
- `nova-spec/20-STDLIB.md` §13: the specification this deviates from, plus
  this increment's own 2026-08-21 amendment; `:27`: the four-name module
  index
- `nova-spec/00-MASTER-SPEC.md:238`: position 8's three names, omitting
  `RwLock`
- `nova-spec/13-RUNTIME.md:129-136`: §4.5's third and also-uncompilable
  spelling of the channel, plus its Tokio claim — recorded above as known
  and out of scope
- `std/sync/lib.nova`: the shipped module, 307 lines, pure Nova
- `crates/nova-runtime/src/task.rs:162`: `enum Wait`, three variants,
  unchanged; `:771`, `:1097`, `:1153`: the three non-exhaustive
  `parked.retain` matches no arm was added to; `:1007`: the sole
  `report_deadlock()` call site, unreachable while any task is runnable
- `crates/nova-resolver/src/lib.rs:695`: `Builtin::STD_ONLY`, 65 entries,
  unchanged; `:796`: `RESERVED_TYPE_NAMES`, 7, unchanged; `:1267`:
  `STD_MODULES`, 12, unchanged
- `tests/runtime/channel_uncontended.nova`, `channel_full_refuses.nova`,
  `channel_fifo_order.nova`, `channel_close_refuses_send.nova`,
  `channel_close_then_drain.nova`: the five synchronous fixtures, all
  `try_send`/`try_recv`
- `tests/runtime/channel_two_tasks_blocking.nova`: the only fixture in
  which `send` and `recv` genuinely suspend and resume; the sole detector
  of both "`send` never blocks" and "`recv` returns `None` when
  empty-but-open", the latter as a **hang**
- `tests/runtime/channel_recv_suspends_only_to_retry.nova` and
  `channel_send_suspends_only_to_retry.nova`: the two placement fixtures.
  Each is the sole detector of its own mutation, and each mutation left
  the whole suite green before these existed — measured, not inferred
