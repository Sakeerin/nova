# Async core — `async`/`await`, `Future<T>` and `std/task` — Design

**Status:** approved 2026-08-07. Phase 2.3a, the first of four slices of spec sub-phase 2.3.

**Base:** `main` at `b067c30` (Phase 2.2e merged and pushed; 688 tests, 18 gate configurations).

---

## 1. Why this, and why now

Everything left in Phase 2 is behind async. `std/fmt` and `std/io` were cut from 2.1 because
every I/O signature in `nova-spec/20-STDLIB.md` §4 is an `async fn`, and all of 2.4
(`net`/`http`/`json`) sits behind those. Nothing else in the phase is blocked on anything else.

The spec's own ordering (`20-STDLIB.md:555`) is `core → fmt/io → collections → strings → fs →
time/log → task → sync → net → http → json → crypto → test`. We have taken `collections`,
`strings` and `test` out of order because each was independently useful. `fmt`/`io` cannot be,
so the deferral has to end here.

### 1.1 Probe table

Every row below was measured against `b067c30`, not inferred from the spec. Two of them
falsify claims the planning documents currently make.

| Claim | Measured | Consequence |
|---|---|---|
| `async fn` needs parser work | **False.** `is_async` already on `FnDecl` (`nova-ast/src/item.rs:49`) and `TraitMethodSig` (`:146`); parsed at `nova-parser/src/grammar.rs:371,400` | Zero parser cost |
| `.await` needs parser work | **False.** `Token::Await` (`nova-lexer/src/token.rs:28`), `Expr::Await` (`nova-ast/src/expr.rs:106`), postfix parse at `grammar.rs:1634-1641` | Zero parser cost |
| `async` is rejected in one place | **False — five sites**, all `E0900`: `nova-typeck/src/check.rs:852` (trait *and* impl method signature loop), `:950` (default-body guard), `:1244` (impl block functions), `:2000` (free functions), `:2086` (extern functions) | The lift is surgical, not a single deletion |
| Thread-per-task is "much simpler to ship" (`docs/phase-2-plan.md` decision 1b) | **False for this codebase.** The GC heap is `thread_local!` — `static HEAP: RefCell<Heap>` (`nova-runtime/src/gc.rs:88`), `alloc` routes through `HEAP.with` (`:128,:155`), `collect()` scans only its own thread's stack base (`:202-212`) | See §2.1. This decision is reversed here |
| `Option`/`Result` are compiler-known types | **False.** They are ordinary prelude sums; the compiler knows no std type by name. There is no lang-item mechanism | `Future<T>` must be a `Ty` variant, not a std record |
| MIR is pre-monomorphization | **False.** `nova_mir::lower_module` (`nova-mir/src/mono.rs:17`) monomorphizes *and* lowers; `Function` is concrete and mangled (`nova-mir/src/lib.rs:67-83`) | The transform sees no generics |
| Highest error code in the `E00xx` band | `E0085` (2.2e took `E0082`–`E0085`). Other bands are in use — `E0207`, `E0403`, `E0601`, `E0900`, `E0902` — but none below `E0100` | `E0086`, `E0087` are free |
| `STD_MODULES` length | 3 | Becomes 4 |

Two further measurements shape §4: `Function` carries a **flat** `temps: Vec<MirTy>` and an
existing env-pointer ABI (`takes_env`, `capture_count`), and `Terminator` already has `Switch`
(`nova-mir/src/lib.rs:426-442`).

---

## 2. The execution model (ADR 0009)

**Single-threaded cooperative state machines.** An `async fn` lowers to a resumable state
machine; all tasks run on one OS thread driven by a cooperative executor. This is what
`nova-spec/13-RUNTIME.md` §4.2 specifies, and it **reverses `docs/phase-2-plan.md` decision 1**,
which recommended thread-per-task over Tokio as the pragmatic choice.

### 2.1 Why the plan's recommendation is wrong here

The plan judged thread-per-task cheaper because it needs almost no compiler work. That is true
in isolation and false in context. Nova's collector keeps its entire heap in a
`thread_local!`: an object allocated on task A lives in A's heap map, and **only A's collector
can see or free it**. Send that object to task B through a channel and A's next collection frees
it while B holds it — B's collector never had it in its map, so B cannot keep it alive either.
Use-after-free, in the subsystem this project has been most careful about.

Making thread-per-task sound therefore requires a global locked heap, stop-the-world
coordination across every thread, all-thread stack scanning, and safepoints. That is a larger
and far subtler body of work than the state-machine lowering it was meant to avoid, and it puts
the risk in the collector rather than in the compiler, where this project's tooling (mutation
testing, `NOVA_GC_STRESS`, adversarial review) is weakest.

Single-threaded state machines leave the thread-local invariant **completely untouched**. The
one GC change needed (§6) is additive and testable.

### 2.2 What is given up

Real parallelism. Nova gets concurrency, not multi-core execution. `spawn_blocking` (spec
`20-STDLIB.md:535`) cannot be honoured and is out of scope. This is a deliberate deviation
recorded in ADR 0009; revisiting it means revisiting the collector first, which is the correct
ordering and was the point.

---

## 3. Scope

**In:** `Ty::Future(Box<Ty>)`; typing of `async fn` and `.await`; the MIR state-machine
transform; a single-threaded executor in `nova-runtime`; a persistent GC root registry;
`block_on`; `async fn main`; `std/task` with `spawn`, `JoinHandle`, `join`, `yield_now`;
`E0086`/`E0087`.

**Out, and each is a later slice or a later phase:** channels and `Mutex` (2.3b), `Instant`/
`Duration`/`sleep`/`timeout` (2.3c), `std/log` (2.3d — fully synchronous in the spec and
independent of all of this), real parking and waking, `spawn_blocking`, cancellation,
async trait methods, async closures (not in the grammar at all), liveness-based state
minimization, and parallelism.

---

## 4. The state-machine transform

A new pass over the monomorphized `Module`, after `lower_module` returns. Both backends consume
`Module`, so the transform lands **once for both**, which `docs/phase-2-plan.md` §5 requires.

For each async `Function`:

1. **State record.** Slot 0 is the resume tag, slot 1 the output, slots 2.. every entry of
   `temps`. All 8-byte slots, allocated `scan = true` — byte-identical in layout to an ordinary
   record, so the collector traces it with no new tracing code.
2. **Poll function.** `takes_env: true`, and the env *is* the state object. Every temp access
   becomes a load/store against the env. This is what buys us out of liveness analysis: **no
   local is live across a suspend by construction**, because no local lives in a register or on
   the stack across one.
3. **Split.** Each block is split at every await. The continuation becomes a new block; the
   suspend stores the resume tag and returns Pending.
4. **Entry.** The entry block becomes a `Switch` on the resume tag — an existing terminator, not
   a new one.
5. **The `Future<T>` value** at the call site is the fat pointer `{poll_code, state_ptr}`, the
   same shape `MakeClosure` already builds for closures.

### 4.1 Why all temps, not just those live across a suspend

Spilling every temp over-retains: a temp dead at every suspend point still occupies a slot and
still roots whatever it points at. That is the correct v1 trade here for three reasons. The
collector is *already* conservative and over-retains by design, so this adds no new failure
mode, only a little more of an accepted one. Liveness analysis is new machinery whose bugs are
exactly the silent kind — a local wrongly judged dead is a use-after-free with no diagnostic.
And it is a pure optimization: adding liveness later narrows the state record and changes no
observable semantics, so it can land on its own evidence.

### 4.2 Poll ABI

```
poll(state_ptr: Ptr, task_ctx: Ptr) -> i64      // 0 = Pending, 1 = Ready
```

The result value is written into the state's output slot rather than returned, so the boundary
to the Rust executor stays a plain scalar and no allocation happens per poll. A heap `Poll<T>`
sum would allocate on **every** poll of **every** task.

`task_ctx` is **unused in 2.3a** and passed as null. It exists now because 2.3b (channels) and
2.3c (timers) both need a task parked against an external event rather than busy re-queued, and
adding a parameter later means rewriting the compiler/runtime boundary and every generated poll
function. One pointer of dead weight now is cheaper than that. The alternative considered and
rejected was encoding the park reason in the return value — that pushes protocol into a magic
integer, which is harder to keep honest than a typed parameter.

---

## 5. Type system

`Ty::Future(Box<Ty>)`, a new variant sitting beside `Ty::Fn`.

Not a std record with a compiler-recognized `DefId`: Nova has no lang-item mechanism (probe
table), and inventing one to describe a type users cannot write or construct is more machinery
for less honesty. A future is *literally* the same fat-pointer shape as a function value, so it
belongs where `Ty::Fn` is.

The cost is breadth. Every `Ty` match arm gains a case: `subst`, `has_params`, `has_vars`,
`has_assoc`, `has_error`, `display_ty`, `mangle_ty`, `mir_ty`, `TyHead`, and unification.
`mir_ty(Future) = MirTy::Ptr`.

**`mangle_ty` gets a real case, and a test.** `Ty::Assoc` collapsing to the string `"X"` already
shipped as a miscompile in this project — two instantiations colliding on one symbol, both
dispatching to the first's code. A placeholder here would reproduce it exactly.

Typing rules:

- `async fn f(A) -> T` has type `fn(A) -> Future<T>`. The declared return type is the *output*,
  as in the spec's signatures.
- `e.await` where `e: Future<T>` has type `T`.
- `.await` is legal only inside an `async fn` body.

### 5.1 Resolving the surface name

`std/task`'s signatures write `Future<T>` in type position, so the name has to resolve. It joins
the built-in type-name table in `convert_ty` — and it is the **first built-in type name that
takes a type argument**; `Int`, `Bool`, `Float`, `Char`, `String` and `Unit` are all nullary, so
the arity path is new and a wrong-arity `Future` or bare `Future` needs its own diagnostic
rather than falling through to "unknown type".

**There are two built-in type-name sites, not one** — `nova-typeck/src/check.rs:2394` and
`:5080` — and both must learn `Future`. This project has already shipped a miscompile from
exactly this shape: the 2.2c structural-match fix reached two of four impl-lookup sites, and the
two it missed were a trait-dispatch path and a bound check that had silently drifted out of
sync. Grep for every site before assuming there are two.

---

## 6. GC integration

This is the one place the collector changes, and the highest-risk item in the slice.

A suspended task's state object is held by the Rust executor. It sits on **no Nova stack** and in
**no register**, which are the only two root sources the collector has. Without a change it is
freed while the task is suspended.

Add a persistent root registry to `gc.rs` — `nova_gc_add_root(ptr)` / `nova_gc_remove_root(ptr)`
— seeded into the mark set before the stack scan. The executor registers each live task's state
object and unregisters it on completion.

**The existing `ROOTS` is not this.** `ROOTS` (`gc.rs:91`) is a scratch buffer filled by
`nova_gc_scan_range` and cleared at the start of every cycle (`:218`, `:222`). The registry must
be a separate, persistent structure; reusing `ROOTS` would drop every task root each collection,
which is precisely the bug being prevented and would look like it worked in any test where no
collection happened at a suspend point.

---

## 7. Diagnostics

| Code | Meaning |
|---|---|
| `E0086` | `.await` outside an `async fn` |
| `E0087` | `.await` applied to a non-future |

Of the five `E0900` sites in the probe table: `:2000` (free functions) and `:1244` (impl block
functions) lift entirely; `:852` becomes conditional, still rejecting trait methods; `:950`
(the default-body guard) and `:2086` (extern functions) are unchanged.

Inherent async methods are **not optional**. The spec's `JoinHandle::join` is
`pub async fn join(self) -> T` (`20-STDLIB.md:539`) — an inherent async method. Rejecting them
would force `join` into a free function, deviating from the spec to save a conditional.

Async trait methods stay rejected: they need associated-type futures, which is a larger design
than this slice, and 2.2c's associated types make it tractable later rather than now.

---

## 8. `std/task`

The fourth embedded std module; `STD_MODULES` goes 3 → 4 (2.2b established that only the array's
length annotation changes — every consumer iterates it).

```nova
pub fn spawn<T>(fut: Future<T>) -> JoinHandle<T>
pub record JoinHandle<T> { /* opaque */ }
impl<T> JoinHandle<T> {
    pub async fn join(self) -> T
}
pub async fn yield_now()
pub fn block_on<T>(fut: Future<T>) -> T
```

`JoinHandle::cancel` (spec `20-STDLIB.md:540`) is out of scope — cancellation needs a parking
model.

`block_on` is a real `std/task` export, not only the runtime's private entry point. It has two
callers: the driver, which wraps an `async fn main` in it (`13-RUNTIME.md:105`), and user code —
including `@test fn t() { block_on(f()) }`, which is how async gets tested without any change to
the test runner (§10). **A re-entrant `block_on` — one called from inside a running executor —
panics** rather than nesting an executor inside a poll. That is a runtime check, not a static
one; catching it statically would need an effect system Nova does not have.

The executor lives in `crates/nova-runtime/src/task.rs`: a task is `{poll_code, state, done}`, a
ready queue drives pop → poll → re-queue-if-Pending, and `block_on` loops until the root task
completes. Round-robin with no wakers, which makes interleaving **deterministic by
construction** — exactly what the sub-phase gate asks for.

---

## 9. Gate

A fixture in which two spawned tasks interleave deterministically at their await points, run
three ways: `nova run` (Cranelift), `nova build` (LLVM object), and `NOVA_GC_STRESS=1`. Gate
configurations go **18 → 21**.

An `async fn main` awaiting a spawned task's `join` must also compile and run under all three.

---

## 10. Testing

Nova-level fixtures under `tests/runtime/async/`, plus `@test` functions where the assertion is
about a value rather than an interleaving. An async test is written
`@test fn t() { block_on(f()) }` — no attribute work and nothing new in the runner.

**Mutation targets are named here, in the spec, rather than discovered in review.** This
project's most expensive repeated lesson is that a test which *exercises* code is not a test
that *discriminates* correct code from broken code — 2.2b shipped eight tasks out of ten where a
one-character mutation survived the task's own tests. Each of these must be killed by the suite:

| Mutation | Must be killed by |
|---|---|
| Change one resume-tag `Switch` arm | a task resuming at the wrong point after its second await |
| Swap the Ready/Pending status constant | a `block_on` that never terminates, or returns early |
| Off-by-one on the output slot index | a task returning the wrong value, not a crash |
| Delete the `nova_gc_add_root` call | `NOVA_GC_STRESS=1` on any test that suspends across an allocation |
| Make `mangle_ty`'s `Future` case a constant | two async functions differing only in output type |

Generic async functions are instantiated at **`Float`**, not Int/Bool. `mir_ty` collapses five
of seven `Ty` variants (`nova-mir/src/lib.rs:445-452`) — `Int`/`Char` both to `I64`, and
`String`/`Fn`/`Sum`/`Record`/`Array` all to `Ptr`, which *is* `i64` on x86-64 — so an
Int-versus-String pair tests nothing. `Float` is `F64` and crosses register banks; `Bool` is
`I8` whose only values survive an `I64` confusion in the low byte.

Run `cargo build --workspace` before `cargo test --workspace --no-fail-fast`. This slice adds
`nova_rt_*` symbols, and `cargo test` does not regenerate `nova-runtime`'s staticlib — the
failure presents as ~25 unrelated `unresolved external symbol` errors that read like a codegen
bug.

---

## 11. Risks

1. **Premature free of a suspended task's state.** The registry (§6) is the whole mitigation,
   and the named mutation above is how we know it works. `NOVA_GC_STRESS=1` on every async test
   is a gate criterion, not belt-and-braces.
2. **`Ty::Future` breadth.** Ten-plus match arms, of which `mangle_ty` has a shipped-miscompile
   precedent. Every arm gets a deliberate case; none gets a catch-all.
3. **The open `0xC0000005` anomaly from 2.2e** (three occurrences, never reproduced in 60+
   targeted runs; see ADR 0008 and the notes in `nova test`'s runner). This slice adds freshly
   linked gate binaries. Its recorded reopen conditions apply unchanged — some-but-not-all
   subprocesses faulting, differing codes between subprocesses of one binary, a trapping test
   still emitting its marker while siblings fault, or any reproduction under `NOVA_GC_STRESS` in
   isolation. If it recurs, **capture the raw exit code**, which the second occurrence failed to
   do.
4. **State-record layout drift.** The transform reproduces codegen's `{slot at 8*i}` record
   layout by hand, the same hazard `str_chars` had in 2.2b as the first intrinsic to construct a
   Nova array in the runtime. Pin it with a test that reads the real tracked `(size, scan)` via
   `gc::object_info`, as 2.2b did.

---

## 12. Definition of done

- `async fn`, `.await`, `Future<T>`, `async fn main`, and `std/task`'s four items work under both
  backends.
- 21 gate configurations green, including the two-task interleaving fixture.
- Every mutation in §10 demonstrably killed.
- `cargo clippy -D warnings` and `cargo fmt --check` clean; full suite green under
  `cargo test --workspace --no-fail-fast`.
- ADR 0009 records the execution model and its reversal of `docs/phase-2-plan.md` decision 1.
- `docs/phase-2-plan.md` decision 1 is **edited in place** to point at ADR 0009. A commit that
  changes a decision must sweep every document asserting the old one — the lesson of the 2.2a
  debt branch, where an enforcement change left three documents contradicting it, two of them
  shipping in the same release.
