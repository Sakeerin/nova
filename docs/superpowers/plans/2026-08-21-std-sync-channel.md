# `std/sync`'s `channel<T>` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a bounded async channel in `std/sync`, completing §13's own specification of the module.

**Architecture:** A private fixed-capacity ring buffer over a flat array, wrapped by a `Channel<T>` that hands out `Sender<T>` and `Receiver<T>` views. Every blocking operation has a non-blocking twin, and blocking is yield-and-retry — no executor change, no new `Wait` variant, zero intrinsics.

**Tech Stack:** Nova (`std/sync/lib.nova`), Rust only for fixture registration (`crates/nova-cli/tests/run_tests.rs`).

**Spec:** `docs/superpowers/specs/2026-08-21-std-sync-channel-design.md`

## Global Constraints

- `cargo build --locked --workspace` **before** `cargo test --locked --workspace --all-features --no-fail-fast`. `--no-fail-fast` is mandatory.
- **44 targets.** Sum **every** `test result:` line. Never pipe through `head`/`tail` before summing — they truncate and silently drop targets. `grep -E 'test result:'` keeps them all.
- Exclude any line containing `trapped` — Nova's own harness schema, which appears only when a fixture **fails**. It never appeared at all in the last increment, including four runs with failing fixtures, because these are `nova run` golden-stdout tests rather than `nova test` harness runs.
- Baseline **1036 / 0 / 8 / 44**. Six new fixtures should give about **1042**. **State the number measured, not predicted.**
- Clippy `--all-targets --all-features -- -D warnings` clean; `cargo fmt --all -- --check` clean. **MSRV 1.78 — no `reason = "..."` in any lint attribute.**
- The **8 ignored ADR-0010 GC tests stay ignored and untouched.**
- The **poll ABI is frozen.** This increment adds **no Rust to the runtime** — the only Rust edit anywhere is fixture registration in `run_tests.rs`.
- `std/sync/lib.nova` is embedded via `include_str!` (`crates/nova-resolver/src/lib.rs`), so editing it **requires a cargo rebuild**. Editing a `.nova` fixture does not — the `nova` CLI compiles those at test time.
- **Every file touched is CRLF**: `std/sync/lib.nova`, every fixture and golden, all markdown under `docs/` and `nova-spec/`. Verify `tr -cd '\r' < f | wc -c` against `wc -l < f` before and after each edit. **`grep` here strips CR from its output**, so `cat -A` shows no `^M` even when present — `od -c` or a byte count is authoritative. **`sed -i` silently stripped all 166 CRs from a `.nova` file last increment**, caught only by a byte count. Prefer the Edit tool.
- `core.autocrlf=true` with no `.gitattributes`: a blob hash legitimately differs from the worktree file, so compare with `git diff`, never by hashing. `git diff` reports **nothing** for a new untracked file — stage it first.
- Commit messages to a UTF-8 file applied with `git commit -F`, **never a heredoc**, each body ending exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA that is not an ancestor of `main`.** `6e834c5` is. The two commits already on this branch are **branch-local and must not be cited** — last increment shipped a constraint that wrongly whitelisted a branch-local SHA.
- Never push, merge, tag, or `git checkout --` anything.

### Measured language facts — rely on these, do not re-derive

All verified at `6e834c5`:

- **No turbofish.** `make<Int>(3)` is `error[P0001]: chained comparison operators are not allowed`. `let ch: Channel<Int> = channel(2)` is the only form. Every fixture must annotate.
- **`%` is a truncating remainder**: `-7 % 3` is `-1`, not `2`.
- Nested field assignment works (`o.inner.n = 5`), nested index assignment works (`o.inner.arr[1] = 9`), and a `mut self` method may be called on a field from a `mut self` method (`self.inner.bump()`). Aliasing writes through — `let mut r = o.inner; r.n = 7` changes `o.inner.n`.
- `[]` typechecks in a generic record (`Vec::new`, `std/collections/lib.nova:15`); `[x; n]` array-repeat works and is how `Vec::push` grows (`:27-37`).
- `Option` **has** `unwrap` (`std/core/lib.nova:26`) plus `is_some`/`is_none`/`map`/`unwrap_or`. Do not route around it.
- **No `loop` keyword** — `loop {` parses as a record literal. Use `while`.
- **`match` arms do not bind `mut`.** Rebind inside the arm: `Some(g) => { let mut held = g ... }`. This is the shape `sync_mutex_uncontended.nova` already uses.
- Records are reference types with **no field privacy** — a forged `Sender { ch: c }` compiles.
- `mut self` receivers are required at field-assignment and method-call sites (`E0060`).
- No `module` header in any of the 13 std `lib.nova` files.

---

## File Structure

| File | Responsibility |
|---|---|
| `std/sync/lib.nova` (modify, currently 166 lines) | Append the channel section after the existing `MutexGuard` impl. Nothing existing changes. |
| `tests/runtime/channel_*.nova` + `.stdout` (create, 6 pairs) | One fixture per behaviour; goldens are CRLF and the harness normalises them. |
| `crates/nova-cli/tests/run_tests.rs` (modify) | Six `#[test]` registrations. **Not automatic** — an unregistered fixture runs zero tests silently. |
| `docs/adr/0017-std-sync-channel-shape.md` (create) | The deviation from §13's signature. |
| `nova-spec/20-STDLIB.md` (modify) | Dated §13 amendment. |
| `CHANGELOG.md` (modify) | `[Unreleased]` Added. |

---

## Task 1: The ring, the records, and the synchronous surface

Independently green with no async whatsoever. A reviewer can reject this without touching Task 2.

**Files:**
- Modify: `std/sync/lib.nova` (append after the file's last line, 166)
- Create: `tests/runtime/channel_uncontended.nova` + `.stdout`
- Create: `tests/runtime/channel_full_refuses.nova` + `.stdout`
- Create: `tests/runtime/channel_fifo_order.nova` + `.stdout`
- Create: `tests/runtime/channel_close_refuses_send.nova` + `.stdout`
- Create: `tests/runtime/channel_close_then_drain.nova` + `.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs` (5 registrations)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces, for Task 2: `Channel<T>` with private `fn push(mut self, v: T) -> Bool` and `fn pop(mut self) -> Option<T>`; `Sender<T> { ch: Channel<T> }`; `Receiver<T> { ch: Channel<T> }`; `pub fn channel<T>(buffer: Int) -> Channel<T>`; `Sender::try_send(mut self, v: T) -> Bool`; `Sender::close(mut self)`; `Receiver::try_recv(mut self) -> Option<T>`; and the field `Channel.closed: Bool`, which Task 2's `send`/`recv` read directly.

- [ ] **Step 1: Write the first fixture and its golden**

`tests/runtime/channel_uncontended.nova`:

```nova
// One task, no contention: fill a capacity-2 channel, drain it in order, and
// confirm it reports empty afterwards. The annotation on `ch` is REQUIRED --
// `channel`'s type parameter appears only in its return type and Nova has no
// turbofish, so `channel(2)` alone gives the checker no way to name `T`.
fn main() {
    let ch: Channel<Int> = channel(2)
    let mut tx = ch.sender()
    let mut rx = ch.receiver()
    println("send1=${tx.try_send(10)}")
    println("send2=${tx.try_send(20)}")
    match rx.try_recv() {
        Some(v) => println("recv1=${v}")
        None => println("unexpected: empty after two sends")
    }
    match rx.try_recv() {
        Some(v) => println("recv2=${v}")
        None => println("unexpected: empty with one value left")
    }
    println("empty=${rx.try_recv().is_none()}")
}
```

`tests/runtime/channel_uncontended.stdout`:

```
send1=true
send2=true
recv1=10
recv2=20
empty=true
```

- [ ] **Step 2: Register it**

Append to `crates/nova-cli/tests/run_tests.rs`, matching the existing shape exactly:

```rust
#[test]
fn channel_uncontended_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/channel_uncontended.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_uncontended.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

- [ ] **Step 3: Run it and watch it fail for the right reason**

Run: `cargo test --locked -p nova-cli channel_uncontended`
Expected: FAIL. The message must name an unknown type or unknown function (`Channel`, `channel`) — **not** a parse error in the fixture. A parse error means the fixture is wrong, not the implementation missing.

- [ ] **Step 4: Append the channel section to `std/sync/lib.nova`**

Append after line 166. Note `head` is not reset in the allocation branch: it is already `0` from construction, and the branch can only run before the first allocation.

```nova

// ------------------------------------------------------------------ channel

// A fixed-capacity FIFO over a flat array. Private: it exists only to give
// `Channel` its buffer, and `std/collections` has no `Queue` to borrow --
// `Vec::pop` takes from the end, which is LIFO.
//
// `head` is the index of the next value out; `len` is how many are live.
// Capacity lives on `Channel` rather than here, because `data` starts empty
// and so cannot report it until the first send allocates.
//
// INVARIANT: `head >= 0`, `len >= 0`, `cap >= 1`. Every `%` below therefore
// has non-negative operands, which matters because Nova's `%` is a
// TRUNCATING remainder -- `-7 % 3` is `-1`, not `2`. Any operation added
// later that seeks backwards must re-establish this, or the wrap arithmetic
// lands outside the array. `std/core/lib.nova:125` and
// `std/collections/lib.nova:119` carry the same warning for hash bucketing.
record Ring<T> { data: [T], head: Int, len: Int }

pub record Channel<T> { ring: Ring<T>, cap: Int, closed: Bool }

// Two views onto one channel. Records are reference types, so both observe
// every mutation; the split carries intent -- hand `tx` to producers and `rx`
// to consumers -- and not enforcement. Nova has no field privacy, so a forged
// `Sender { ch: c }` compiles and can send or close on a channel it was never
// given. Documented, not prevented; the same hazard `MutexGuard` carries.
pub record Sender<T> { ch: Channel<T> }
pub record Receiver<T> { ch: Channel<T> }

// A bounded channel holding at most `buffer` values.
//
// `buffer < 1` clamps to 1. A zero-capacity rendezvous channel needs a
// handoff protocol that yield-and-retry cannot express without a second wait
// state, and clamping follows `Int::pad`'s precedent of an early return over
// a panic.
//
// The call site must annotate: `let ch: Channel<Int> = channel(2)`. The type
// parameter appears only in the return type, and Nova has no turbofish --
// `channel<Int>(2)` is a parse error, read as two comparisons.
pub fn channel<T>(buffer: Int) -> Channel<T> {
    let cap = if buffer < 1 { 1 } else { buffer }
    Channel { ring: Ring { data: [], head: 0, len: 0 }, cap: cap, closed: false }
}

impl<T> Channel<T> {
    pub fn sender(self) -> Sender<T> { Sender { ch: self } }

    pub fn receiver(self) -> Receiver<T> { Receiver { ch: self } }

    // Enqueue, or report refusal. Private: `Sender::try_send` is the public
    // door. Returns false when closed and when full, which is why the async
    // `send` must re-check `closed` itself to tell the two apart.
    fn push(mut self, v: T) -> Bool {
        if self.closed { return false }
        if self.ring.len == self.cap { return false }
        if self.ring.data.len() == 0 {
            // First send. `[fill; n]` needs a value of `T` and a fresh
            // channel has none, so allocation waits for one -- the same trick
            // `Vec::push` uses to grow. Every slot holds `v` until
            // overwritten, so the channel briefly retains up to `cap`
            // references to the first value sent.
            self.ring.data = [v; self.cap]
            self.ring.len = 1
            return true
        }
        self.ring.data[(self.ring.head + self.ring.len) % self.cap] = v
        self.ring.len = self.ring.len + 1
        true
    }

    // Dequeue, or None when empty. Says nothing about `closed` -- that
    // distinction belongs to `Receiver::recv`.
    fn pop(mut self) -> Option<T> {
        if self.ring.len == 0 { return None }
        let v = self.ring.data[self.ring.head]
        self.ring.head = (self.ring.head + 1) % self.cap
        self.ring.len = self.ring.len - 1
        Some(v)
    }
}

impl<T> Sender<T> {
    // Send if there is room. False when the channel is full OR closed; a
    // caller that needs to distinguish them should use `send`, which returns
    // false only for closed.
    pub fn try_send(mut self, v: T) -> Bool {
        self.ch.push(v)
    }

    // Refuse all further sends. Buffered values stay readable, so a producer
    // may close the instant it stops producing and the consumer still drains.
    // Idempotent: a second call sets an already-set flag.
    pub fn close(mut self) {
        self.ch.closed = true
    }
}

impl<T> Receiver<T> {
    // Take a value if one is buffered. None means empty, and says nothing
    // about whether more will arrive -- `recv` is what distinguishes those.
    pub fn try_recv(mut self) -> Option<T> {
        self.ch.pop()
    }
}
```

- [ ] **Step 5: Run it and watch it pass**

Run: `cargo build --locked --workspace` then `cargo test --locked -p nova-cli channel_uncontended`
Expected: PASS. Remember the rebuild is required — `lib.nova` is embedded via `include_str!`.

- [ ] **Step 6: Add the remaining four synchronous fixtures**

`channel_full_refuses.nova` — a full channel refuses, and accepts again after one is taken:

```nova
fn main() {
    let ch: Channel<Int> = channel(1)
    let mut tx = ch.sender()
    let mut rx = ch.receiver()
    println("first=${tx.try_send(1)}")
    println("second_full=${tx.try_send(2)}")
    println("drained=${rx.try_recv().unwrap_or(0)}")
    println("after_drain=${tx.try_send(3)}")
}
```

Golden: `first=true`, `second_full=false`, `drained=1`, `after_drain=true`.

`channel_fifo_order.nova` — order survives a wrap. Capacity 2, six values through, which wraps `head` three times:

```nova
fn main() {
    let ch: Channel<Int> = channel(2)
    let mut tx = ch.sender()
    let mut rx = ch.receiver()
    let mut out = ""
    let mut next = 1
    while next <= 6 {
        let sent = tx.try_send(next)
        if !sent { println("unexpected: refused ${next}") }
        if next % 2 == 0 {
            out = "${out}${rx.try_recv().unwrap_or(0)} ${rx.try_recv().unwrap_or(0)} "
        }
        next = next + 1
    }
    println("order=${out}")
    println("empty=${rx.try_recv().is_none()}")
}
```

Golden: `order=1 2 3 4 5 6 `, `empty=true`. **The trailing space is intentional** — match the golden to what the code prints, byte for byte.

`channel_close_refuses_send.nova` — close refuses, and is idempotent:

```nova
fn main() {
    let ch: Channel<Int> = channel(2)
    let mut tx = ch.sender()
    println("before=${tx.try_send(1)}")
    tx.close()
    println("after=${tx.try_send(2)}")
    tx.close()
    println("after_second_close=${tx.try_send(3)}")
}
```

Golden: `before=true`, `after=false`, `after_second_close=false`.

`channel_close_then_drain.nova` — buffered values survive close:

```nova
fn main() {
    let ch: Channel<Int> = channel(3)
    let mut tx = ch.sender()
    let mut rx = ch.receiver()
    println("s1=${tx.try_send(7)}")
    println("s2=${tx.try_send(8)}")
    tx.close()
    println("d1=${rx.try_recv().unwrap_or(0)}")
    println("d2=${rx.try_recv().unwrap_or(0)}")
    println("then_empty=${rx.try_recv().is_none()}")
}
```

Golden: `s1=true`, `s2=true`, `d1=7`, `d2=8`, `then_empty=true`.

Register all four exactly as in Step 2, substituting each name.

- [ ] **Step 7: Run the five and confirm all pass**

Run: `cargo test --locked -p nova-cli channel_`
Expected: 5 passed, 0 failed.

- [ ] **Step 8: Mutate, and COUNT across the whole suite**

For each mutation: apply it, `cargo build --locked --workspace`, run the **whole** suite, record **how many** tests fail and **which**, then restore and confirm the restore with `git status` and `git diff` plus a CR/LF byte count on `lib.nova`.

| Mutation | Site |
|---|---|
| `push` returns `true` when full | delete `if self.ring.len == self.cap { return false }` |
| dequeue from the tail | `pop` reads `self.ring.data[(self.ring.head + self.ring.len - 1) % self.cap]` |
| `close` stops refusing | delete `if self.closed { return false }` from `push` |
| `head` advances without the modulo | `self.ring.head = self.ring.head + 1` |

**Do not claim any fixture is the only one that catches its mutation.** Four such claims were measured false across the last four increments, and one shipped inside the correction of another. Report the measured count, and say whether your list of affected tests is exhaustive or partial.

- [ ] **Step 9: Full verification**

Run `cargo build --locked --workspace`, then `cargo test --locked --workspace --all-features --no-fail-fast`, summing every `test result:` line. Then clippy and fmt per Global Constraints. Expect 1036 + 5 = **1041**; state what you measure.

- [ ] **Step 10: Commit**

Write the message to a UTF-8 file and apply with `git commit -F`. Stage `std/sync/lib.nova`, the ten fixture files, and `run_tests.rs`.

---

## Task 2: The async twins and the load-bearing fixture

**Files:**
- Modify: `std/sync/lib.nova` (add `send` to the `Sender` impl, `recv` to the `Receiver` impl)
- Create: `tests/runtime/channel_two_tasks_blocking.nova` + `.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs` (1 registration)

**Interfaces:**
- Consumes: everything Task 1 produced, especially `Channel.closed`, `Channel::push` and `Channel::pop`.
- Produces: `Sender::send(mut self, v: T) -> Bool` (async) and `Receiver::recv(mut self) -> Option<T>` (async).

- [ ] **Step 1: Write the load-bearing fixture and its golden**

`tests/runtime/channel_two_tasks_blocking.nova`:

```nova
// ORDER DEPENDENCE -- read this before editing.
//
// This fixture depends on the executor's ready queue being FIFO round-robin
// (`crates/nova-runtime/src/task.rs:184`): a task re-queued by `yield_now`
// waits behind every task already waiting. That is a documented runtime
// invariant, not an observed coincidence, but it is what makes the
// interleaving below deterministic.
//
// The capacity is 2 and the producer sends 5, and BOTH numbers are load
// bearing. N > cap forces `send` to suspend, so a `send` that never blocks
// loses values. N > cap also forces `head` to wrap twice, so a missing
// modulo corrupts the output -- a fixture with N <= cap never reaches that
// path at all.
async fn produce(mut tx: Sender<Int>) {
    let mut next = 1
    while next <= 5 {
        let sent = tx.send(next).await
        if !sent { println("unexpected: send refused ${next}") }
        next = next + 1
    }
    tx.close()
}

async fn consume(mut rx: Receiver<Int>) {
    let mut out = ""
    let mut more = true
    while more {
        match rx.recv().await {
            Some(v) => out = "${out}${v} "
            None => more = false
        }
    }
    println("${out}done")
}

async fn main() {
    let ch: Channel<Int> = channel(2)
    let a = spawn(produce(ch.sender()))
    let b = spawn(consume(ch.receiver()))
    a.join().await
    b.join().await
}
```

Golden `tests/runtime/channel_two_tasks_blocking.stdout`:

```
1 2 3 4 5 done
```

- [ ] **Step 2: Register it** — same shape as Task 1 Step 2, name `channel_two_tasks_blocking`.

- [ ] **Step 3: Run it and watch it fail for the right reason**

Run: `cargo test --locked -p nova-cli channel_two_tasks_blocking`
Expected: FAIL naming an unknown method `send` or `recv` — not a parse error.

- [ ] **Step 4: Add `send` to the `Sender` impl**

```nova
    // Send, waiting while the channel is full. Returns false only when the
    // channel is closed -- unlike `try_send`, which also returns false when
    // merely full.
    //
    // Waiting is `yield_now`, not parking, which keeps this module in Nova
    // with no executor change (ADR 0016 made the same call for `Mutex`). The
    // cost is stated rather than hidden: a waiter stays *runnable*, so
    // `report_deadlock` cannot see it, and a producer blocked on a channel
    // nobody drains spins instead of being diagnosed.
    pub async fn send(mut self, v: T) -> Bool {
        while !self.ch.push(v) {
            if self.ch.closed { return false }
            yield_now().await
        }
        true
    }
```

- [ ] **Step 5: Add `recv` to the `Receiver` impl**

`while`, not `loop` — Nova has no `loop` keyword.

```nova
    // Take the next value, waiting while the channel is empty and open.
    //
    // None means closed AND drained, never merely empty. That is the whole
    // reason `close` is explicit: `Drop` is specified in three spec files and
    // implemented in none, so nothing closes a channel on the producer's
    // behalf, and without this distinction a consumer loop could not
    // terminate.
    //
    // Buffered values are drained before the close is reported, because `pop`
    // is tried before `closed` is read on every iteration.
    pub async fn recv(mut self) -> Option<T> {
        let mut out = self.ch.pop()
        while out.is_none() {
            if self.ch.closed { return None }
            yield_now().await
            out = self.ch.pop()
        }
        out
    }
```

- [ ] **Step 6: Run it and watch it pass**

Run: `cargo build --locked --workspace` then `cargo test --locked -p nova-cli channel_`
Expected: 6 passed, 0 failed.

- [ ] **Step 7: Prove the fixture bites, and count**

Two mutations, each applied, built, run against the **whole** suite, counted, then restored and the restore verified:

| Mutation | Expected |
|---|---|
| `send` never blocks — body becomes `self.ch.push(v)` | values are lost; this fixture fails |
| `recv` returns `None` on empty-but-open — delete the `closed` check and return `None` | output truncates |

Then run the Task 1 mutation table again now that six fixtures exist, because **adding a fixture changes the population a uniqueness claim is quantified over.** The last increment's re-review made exactly this point after a claim was re-endorsed against a stale population.

- [ ] **Step 8: Confirm the `yield_now` placement is load-bearing**

Move `yield_now().await` in `recv` to before the `pop`, rebuild, and record what happens. On the `Mutex` increment the equivalent line's placement was the entire power of the branch's only real test, and a later edit that moved it would have silently reduced the fixture to a shape test. Record the finding either way.

- [ ] **Step 9: Full verification** — as Task 1 Step 9. Expect **1042**; state what you measure.

- [ ] **Step 10: Commit.**

---

## Task 3: Records

No code. An unchanged test total is the evidence.

**Files:**
- Create: `docs/adr/0017-std-sync-channel-shape.md`
- Modify: `nova-spec/20-STDLIB.md` (dated §13 amendment)
- Modify: `CHANGELOG.md` (`[Unreleased]` Added)

**Interfaces:** Consumes the shipped surface from Tasks 1 and 2. Produces nothing code-facing.

- [ ] **Step 1: Confirm the ADR number is free**

Run: `ls docs/adr/`
Expected: `0001`–`0016` in use, so `0017` is next. A previous increment guessed a number already taken — check, do not assume.

- [ ] **Step 2: Write ADR 0017**

It must record:

1. **The deviation.** §13 writes `channel<T>(buffer: Int) -> (Sender<T>, Receiver<T>)`; tuples are `error[E0900]`, whose own note says the feature "arrives in a later milestone". The shipped signature returns `Channel<T>` — the record §13 declares immediately above the function and never returns — with `sender()`/`receiver()` accessors. All three of §13's type names are now built; only the return type moved, and it moved *to* a spec type.
2. **That reverting is optional, not owed.** When tuples land, the accessor form remains defensible: it gives the pair an identity that can carry future accessors without changing the signature. Say this explicitly so a later reader does not "restore" the tuple believing that was always the intent.
3. **The annotation requirement.** `let ch: Channel<Int> = channel(2)` is the only available call form, because `T` appears only in the return type and Nova has no turbofish (`channel<Int>(2)` is a parse error, read as two comparisons). Record the two rejected escapes: a seed parameter would make `T` inferable and delete the lazy-allocation branch, but promotes an implementation detail to API; `Channel::new` has the identical inference problem.
4. **Yield-and-retry, again.** Consistent with ADR 0016 for `Mutex` — no fourth `Wait` variant, no arm in any of the three non-exhaustive `retain` matches in `task.rs`. The cost: a waiter stays runnable, so `report_deadlock` cannot see a channel nobody serves.
5. **What is NOT closed.** Three documents disagree on position 8's contents — `nova-spec/20-STDLIB.md:27` names four (`Mutex, RwLock, channel, atomic`), `nova-spec/00-MASTER-SPEC.md:238` names three and omits `RwLock`, and §13 specifies only two. State all three. The honest claim is that **`channel` completes §13's own specification of `std/sync`** while `RwLock` and `atomic` remain named in index lines no section specifies. **Position 8 stays partial — do not describe this increment as closing it.**
6. **ADR 0016's blocker, superseded by cross-reference.** ADR 0016 lists the tuple blocker at `:154-155` and again at `:195`. **Do not edit those lines** — a dated ADR states what was true at its date. Record here that the blocker is resolved by redesign.

- [ ] **Step 3: Amend `nova-spec/20-STDLIB.md` §13**

House style is `**AMENDED <date> (branch \`<branch>\`):**`. **Regrep the live marker set** — `grep -n 'AMENDED' nova-spec/20-STDLIB.md` — and **say whether the list you give is exhaustive or partial.** Last increment's correction replaced an exhaustive list with a truncated one and so invited the very copy-forward it existed to stop.

The amendment records: the shipped surface; the signature deviation and its measured cause; that `None` from `recv` means closed and drained; the `buffer < 1` clamp; the annotation requirement; the two documented-not-enforced hazards (invisible spin, forgeable handles); and that `RwLock` and `atomic` remain.

- [ ] **Step 4: CHANGELOG `[Unreleased]` Added**

Follow the file's existing conventions and wrap to its prevailing width — two lines went out unwrapped at 119 chars last increment and became the file's longest.

- [ ] **Step 5: Verify no claim was left standing**

After writing each record, **grep for the consequences of any claim you changed, not its wording.** Searching a retracted phrase finds only the sentence you already fixed. Five times on this project a claim was corrected at one site while another repeating it stood; three consecutive fix waves last increment each shipped a fresh false claim this way. Specifically check nothing anywhere still says `channel` is blocked or unbuilt.

- [ ] **Step 6: Full verification** — expect **1042 / 0 / 8 / 44, unchanged**. For a records-only task an unchanged total is the evidence. CR/LF byte counts on all three files before and after.

- [ ] **Step 7: Commit.**

---

## Self-Review

**Spec coverage.** §1 motivation → ADR 0017 Step 2.1. §2 deviation → Task 3 Step 2.1–2.2. §3 surface → Tasks 1 and 2. §3.1 annotation → every fixture, plus ADR 0017 Step 2.3. §4 ring and the negative-`%` invariant → Task 1 Step 4's comment. §5 semantics (`None` = closed and drained, close-then-drain, `Bool` return, clamp) → Task 1 fixtures `channel_close_then_drain`/`channel_close_refuses_send`, Task 2's `recv`. §6 hazards → doc comments in Task 1 Step 4 and Task 2 Step 4. §7 testing → Task 1 Steps 6–8, Task 2 Steps 7–8. §8 records → Task 3. §9 rejected alternatives → ADR 0017 Step 2.3.

**Gap found and closed:** §4's retention note (a channel holds up to `cap` references to the first value sent) had no home; it is now in Task 1 Step 4's `push` comment.

**Type consistency.** `push`/`pop` are private on `Channel<T>` and used by `try_send`/`try_recv` in Task 1 and by `send`/`recv` in Task 2 — same names, same signatures in both. `Channel.closed` is read by Task 2's `send` and `recv` and written only by `Sender::close`. Fixtures use `try_send`/`try_recv`/`close`/`send`/`recv` and `sender()`/`receiver()` throughout, with no name appearing in two forms.

**Placeholder scan:** none.
