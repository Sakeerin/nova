# `std/sync`'s `channel<T>` — design

**Status:** approved 2026-08-21. Branch: `std-sync-channel`. BASE `6e834c5`.

Builds the second of the two types `nova-spec/20-STDLIB.md` §13 declares for `std/sync`.
`Mutex<T>` shipped in PR #22 (ADR 0016); this closes `Channel<T>`.

---

## 1. Why now, and what it is worth

Channels are not a peripheral item. `nova-spec/00-MASTER-SPEC.md:55` names
"**Channels** for message passing (`std/sync/channel`)" among the language's top-level
features, at the same altitude as the phase goals — not merely inside a module list.

ADR 0016 deferred it with a measured blocker: §13 writes the constructor as returning a
tuple and Nova has no tuples. That blocker is real but **cosmetic** — nothing about a
channel needs a tuple. This increment removes it by choosing a return type the language
can express, and §13 turns out to already declare exactly the right one.

## 2. The deviation, and why it is small

§13 declares three types and returns two of them:

```
pub record Channel<T> { /* opaque */ }
pub fn channel<T>(buffer: Int) -> (Sender<T>, Receiver<T>)
```

The tuple is unexpressible. **Measured** at `6e834c5`, compiling
`fn pair() -> (Int, Bool)`:

```
error[E0900]: tuple types are not supported yet
  = the Phase 1 MVP compiler supports a subset of Nova; this feature arrives in
    a later milestone
```

Note what the diagnostic says: tuples **arrive in a later milestone**. This is a gap,
not a decision.

**The deviation: `channel<T>(buffer: Int) -> Channel<T>`,** with `sender()` and
`receiver()` accessors. This is the smallest available change, because it invents no
name — `Channel<T>` is the record §13 itself declares and then never returns. All three
of §13's type names get built; only the return type moves, and it moves *to* a spec
type rather than away from one.

**When tuples land, reverting is a choice and not an obligation.** The accessor form is
arguably better than the tuple: it gives the pair an identity, so an accessor like
`is_closed()` or `len()` could later be added without changing the signature or
inventing a third return value. Neither ships here — nothing in this design needs them,
and untested public API is worse than absent API. ADR 0017 should say this explicitly
rather than filing the design as a stopgap, so a future reader does not "restore" the
spec signature believing that was always the intent.

## 3. API surface

```nova
record Ring<T> { data: [T], head: Int, len: Int }          // private, not exported

pub record Channel<T> { ring: Ring<T>, cap: Int, closed: Bool }
pub record Sender<T>   { ch: Channel<T> }
pub record Receiver<T> { ch: Channel<T> }

pub fn channel<T>(buffer: Int) -> Channel<T>

impl<T> Channel<T> {
    pub fn sender(self) -> Sender<T>
    pub fn receiver(self) -> Receiver<T>
}

impl<T> Sender<T> {
    pub fn try_send(mut self, v: T) -> Bool        // false when full or closed
    pub async fn send(mut self, v: T) -> Bool      // yields while full; false if closed
    pub fn close(mut self)                          // idempotent
}

impl<T> Receiver<T> {
    pub fn try_recv(mut self) -> Option<T>          // None when empty
    pub async fn recv(mut self) -> Option<T>        // None only when closed AND drained
}
```

**Every blocking operation has a non-blocking twin.** That is not novelty; it is the
shape `Mutex` already ships — `try_lock` beside `async lock` — and the module should
look like itself.

`Sender` and `Receiver` both hold the same `Channel<T>`. Records are reference types, so
both observe every mutation; the split carries intent (hand `tx` to producers, `rx` to
consumers), not enforcement.

### 3.1 Call sites need a type annotation, and this is forced

`T` appears only in `channel`'s **return** type, so nothing infers it from the
argument. **Measured at `6e834c5`: Nova has no turbofish.** `make<Int>(3)` does not
parse -- the parser reads `make < Int > (3)` and reports
`error[P0001]: chained comparison operators are not allowed (use parentheses)`. An
annotated `let` does supply the parameter:

```nova
let ch: Channel<Int> = channel(2)      // works
let ch = channel(2)                    // no way to name T
```

So **every call site must annotate**, and every fixture and doc example must show the
annotated form. This is a real ergonomic cost, and it is worth naming why the obvious
escape was refused: a seed parameter (`channel(2, 0)`) would make `T` inferable *and*
delete the lazy-allocation branch in §4 outright, but it puts a parameter in the public
signature that exists only because `[fill; n]` needs a value -- an implementation detail
promoted to API. The annotation is the smaller price, and Rust's own `mpsc` often needs
the same annotation when the element type is otherwise unconstrained.

`Channel::new(buffer)` as an associated function was also considered and is no better:
the inference problem is identical, so it would need the same annotation while moving
further from §13's `channel` free function.

## 4. The ring buffer

```
enqueue: if len == cap  -> reject
         data[(head + len) % cap] = v ; len += 1
dequeue: if len == 0    -> None
         v = data[head] ; head = (head + 1) % cap ; len -= 1 ; Some(v)
```

`%` exists and is in use — `std/time/lib.nova:251` and `:254-256`.

**But Nova's `%` is a truncating remainder and can return negative.** Measured at
`6e834c5`: `-7 % 3` prints `-1`. Both `std/core/lib.nova:125` and
`std/collections/lib.nova:119` carry comments warning about exactly this for hash
bucketing. The ring is safe by invariant rather than by luck: `head` and `len` are
never negative and `cap >= 1` always holds, so both expressions above have
non-negative operands. **State that invariant in the code**, because it is the
precondition any future backwards-seeking operation would break.

**Allocation is lazy, and this is forced rather than chosen.** `[fill; n]` needs a value
of `T`, and a fresh channel has none. So `Ring` starts `data: []` and allocates
`[v; cap]` on the first send. Both halves are proven by shipped, tested code rather than
assumed: `Vec::new` writes `Vec { len: 0, data: [] }` (`std/collections/lib.nova:15`),
so an empty array literal typechecks in a generic record; and `Vec::push`
(`:27-37`) seeds a fresh array with the incoming element exactly this way.

`data.len() == 0` is the "not yet allocated" test. It is unambiguous because `cap >= 1`
always holds (§5), so an allocated ring's array is never empty.

**One consequence worth documenting:** `[v; cap]` fills every slot with the first value
sent, so a channel retains up to `cap` references to that value until they are
overwritten. This is inherited from `Vec::push`'s growth and is a retention detail, not
a correctness one.

## 5. Semantics

**`recv` returning `None` means closed *and* drained — never merely empty.** This is the
load-bearing decision. It is the only signal that lets a consumer's `while` loop
terminate, and it is why `close()` must be explicit: `Drop` is specified in three places
(`nova-spec/12-TYPESYSTEM.md:192`, `nova-spec/13-RUNTIME.md:96`,
`nova-spec/14-CODEGEN.md:24`) and implemented in none — `std/core` contains zero
occurrences of `trait Drop`. Nothing closes a channel on the producer's behalf.

**Buffered values survive close.** `close()` refuses further sends but leaves the ring
readable, so a producer may close the instant it stops producing and the consumer still
drains. Without this, close would be a race rather than a handshake.

**`send` returns `Bool` rather than panicking** on a closed channel, matching
`try_lock`'s decision to make refusal a value. `Option` is not needed here and
`Option::unwrap` does exist (`std/core/lib.nova:26`) — the earlier claim that it does
not was false, and no design on this branch should route around it.

**`buffer < 1` clamps to 1.** A zero-capacity rendezvous channel needs a handoff
protocol that yield-and-retry cannot express without a second flag and a second wait
state. `Int::pad`'s early return is the precedent for clamping over panicking.

## 6. Two hazards, documented and not fixed

Both are inherited from `Mutex` and both would require touching the executor to fix,
which this increment refuses to do for the same reason ADR 0016 gave.

**A never-closed channel spins invisibly.** `recv` waits by `yield_now().await`, so the
waiter stays *runnable* — `report_deadlock` cannot see it, and a consumer awaiting a
producer that forgot to close burns turns rather than being diagnosed. The ready queue
is FIFO round-robin (`crates/nova-runtime/src/task.rs:184`), so it starves nothing; it
simply never terminates.

**Handles are forgeable.** Nova enforces no field privacy, so
`Sender { ch: some_channel }` compiles and can send or close on a channel it was never
handed. This is the same class of hazard `MutexGuard` documents, and it is stated once,
plainly, rather than implied.

## 7. Testing

**One fixture carries most of the weight:** capacity 2, a producer sending 1…5 and then
closing, a consumer receiving until `None`. Golden `1 2 3 4 5 done`.

That shape is chosen because sending 5 into a buffer of 2 pins four things at once:

1. **`send` must block** — the producer suspends twice, so a non-blocking `send` loses
   values.
2. **The ring must wrap** — `head` passes the end twice, so a missing modulo corrupts
   output. A fixture with `N <= cap` never reaches that path at all.
3. **FIFO order** — a tail-dequeue prints reversed.
4. **Drain-then-`None`** — a `recv` that reports `None` while open truncates.

Supporting fixtures, all synchronous: `try_send` on a full channel is `false`;
`try_recv` on an empty one is `None`; send-after-close is `false`; `close` is idempotent;
buffered values remain receivable after close.

**Mutations to run and report:** `try_send` returning true when full; dequeue from tail
instead of head; `recv` returning `None` on empty-but-open; `close` not refusing later
sends; `head` incremented without the modulo.

**Run every mutation against the whole suite and count.** Do not claim any fixture is
the only one that catches its mutation — four such claims were measured false across the
last four increments, and one of them shipped inside the correction of another. State
the number measured, and state whether a list is exhaustive or partial.

Both async fixtures depend on the FIFO ready queue and so must carry the
`ORDER DEPENDENCE -- read this before editing` block the `Mutex` fixtures use, citing
`task.rs:184`. **Fixture registration is not automatic** — every `tests/runtime/*.nova`
needs an explicit `#[test]` in `crates/nova-cli/tests/run_tests.rs` or it silently runs
zero tests.

Baseline is 1036 / 0 / 8 / 44. Six new fixtures should give about 1042; state the number
measured rather than the number predicted.

## 8. Records

- **ADR 0017** — the deviation and its reasoning (§2), including that reverting to the
  tuple signature is optional rather than owed.
- **`nova-spec/20-STDLIB.md` §13** — a dated amendment in the file's house style,
  `**AMENDED <date> (branch \`<branch>\`):**`. Regrep the live set of markers; do not
  copy a list forward, and say whether the list you give is exhaustive.
- **`CHANGELOG.md`** `[Unreleased]` Added.
- **ADR 0016 must be superseded by cross-reference, not edited.** It lists the tuple
  blocker at `:154-155` and again at `:195`. A dated ADR states what was true at its
  date; the correct move is a note in ADR 0017 recording that this blocker is resolved
  by redesign.

### What this does NOT close

Three documents disagree on what position 8 contains, and an ADR on the previous
increment was found to have mixed position-7 items into it. So state all three:

| Source | Names for `std/sync` |
|---|---|
| `nova-spec/20-STDLIB.md:27` (§1 index) | `Mutex, RwLock, channel, atomic` — four |
| `nova-spec/00-MASTER-SPEC.md:238` (§3 position 8) | `Mutex, channel, atomic` — three, no `RwLock` |
| `nova-spec/20-STDLIB.md` §13 (the specification) | `Mutex` and `Channel` — two |

The honest claim is therefore narrow and should be made in exactly these terms:
**`channel` completes §13's own specification of `std/sync`**, both declared types being
built. `RwLock` and `atomic` remain named in index lines that no section specifies.
Position 8 stays **partial**. This increment must not be described as closing it.

## 9. Rejected alternatives

- **A named pair record** (`ChannelPair<T> { tx, rx }`) — the most mechanical
  translation of the tuple, but it invents a name §13 does not have and leaves
  `Channel<T>` declared and unused.
- **One `Channel<T>` with `send`/`recv` directly on it** — fewest moving parts, but
  discards the producer/consumer split that is the point of the API and leaves two
  spec-declared types unbuilt.
- **`Queue<T>` in `std/collections`** — would also close §1's long-standing `Queue`
  gap, but it makes a position-8 increment touch position 3's public API, with its own
  records, tests and ADR surface. Deferred deliberately; the ring stays private.
- **`Vec<T>` with a shift-down dequeue** — simplest code, but an O(n) receive inside the
  concurrency module is a trap, and `Vec::pop` is LIFO so it cannot be used as-is.
- **Parking instead of yield-and-retry** — would let `report_deadlock` see a stalled
  consumer, but needs a fourth `Wait` variant and an arm in each of the three
  non-exhaustive `retain` matches in `task.rs` (`wake_tasks_waiting_on:771`,
  `wake_ready:1097`, `wake_due:1153`), where an omitted arm parks a task forever with no
  compile error and no diagnostic. ADR 0016 declined this for `Mutex`; declining it
  again here is consistency, not inertia.
