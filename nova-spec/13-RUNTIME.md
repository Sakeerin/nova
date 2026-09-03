# 13 — Runtime Specification

> Crate: `nova-runtime`
> Phase: 1 (skeleton), 2 (full features)
> Linked into every Nova binary

---

## 1. Architecture

The runtime is a **Rust crate** that gets statically linked into every Nova binary. Compiled Nova code calls Rust functions for:
- GC allocation
- Async scheduling
- Panic handling
- I/O primitives
- FFI marshalling

This leverages the Rust ecosystem rather than re-implementing it — with one
deliberate exception: the async executor is hand-written and single-threaded (4.1),
not a wrapped Tokio.

```
+-----------------------------------------+
|  Compiled Nova Code (LLVM-generated)    |
|    calls into ↓                         |
+-----------------------------------------+
|  nova-runtime (Rust)                    |
|    - GC: conservative mark-sweep (3.1)  |
|    - Cooperative executor (4.1)         |
|    - Hyper HTTP                         |
|    - Ring crypto                        |
|    - Allocator                          |
|    - Panic infrastructure               |
+-----------------------------------------+
|  OS (libc)                              |
+-----------------------------------------+
```

Of the components above, the executor, allocator, GC and panic infrastructure are
built. **Hyper HTTP and Ring crypto are not** — `std/http` and `std/crypto` are
unstarted, and the workspace depends on neither crate. No third-party collector is
used either — the GC is hand-written (3.1).

**AMENDED 2026-09-01 (branch `std-http-parsing`): `std/http` is no longer
unstarted, and "Hyper HTTP" above was never quite the right label for what it
would ship as.** The server half — request-head parsing plus response
serialisation — ships over one intrinsic and `httparse` 1.10.1, not over
hyper: hyper's own executor and connection driver cannot run on this runtime
for three measured reasons (`docs/adr/0019-offset-table-intrinsic-boundary.md`).
So the diagram box above is best read as "HTTP parsing (`httparse`)" going
forward rather than "Hyper HTTP" — a box this increment fills differently
than the label predicted, not one it leaves empty. `std/crypto` is the one
box in that diagram still unbuilt and the workspace still depends on neither
`hyper` nor `ring`.

---

## 2. Memory Layout

### 2.1 Object metadata — a side table, not a header

**AMENDED 2026-08-22 (branch `spec-runtime-async-truth`): this section specified an
in-band `ObjectHeader { type_id: u32, flags: u32 }` preceding every object's fields.
There is no header.** Corrected here because leaving it would contradict 3.3 below,
which records that the allocator is given no type identity — the two claims cannot both
stand.

`alloc` returns a bare pointer to the requested bytes, with **nothing in front of them**.
The collector keeps its metadata out of band, in a side table
(`crates/nova-runtime/src/gc.rs`):

```rust
struct Obj {
    addr: usize,    // address returned to the mutator
    size: usize,    // allocation size in bytes
    scan: bool,     // trace this object's words, or treat it as a leaf
    marked: bool,   // set during the mark phase
}
```

So there is **no `type_id` anywhere**, in the header sense or any other: a conservative
collector (3.1) has no use for type identity, which is why `alloc` takes `scan: bool`
instead. And there is no in-band mark word — `marked` lives in the side table beside the
address.

One consequence worth stating, since an in-band header would have implied otherwise: an
object's address is the *whole* of its identity to the collector, and 3.1 records that a
sweep really frees, so addresses are reused. Nothing may key a persistent table on one.

### 2.2 Primitive Layout
- `Int` (default) = `i64`, stack-allocated when possible
- `Float` (default) = `f64`
- `Bool` = `i8`
- `Char` = `u32` (Unicode scalar)
- `String` = heap object: `{ len: usize, data: ptr<u8> }` UTF-8
- Tuple = struct of fields, no header if monomorphized & on stack
- Records = boxed by default (header + fields)
- Sum types = tagged union: `{ tag: u32, payload: union { ... } }`

### 2.3 String Encoding
- UTF-8, immutable
- Slicing returns `Str` (view) — no copy
- `String` (owned) vs `Str` (borrowed) — like Rust's `String`/`&str`

---

## 3. Garbage Collector

**AMENDED 2026-08-22 (branch `spec-runtime-async-truth`): this section named two
third-party collectors, neither of which is used, and declared a compiler-facing
interface of four functions, none of which exists in any form.** The collector is
hand-written. Rewritten below against `crates/nova-runtime/src/gc.rs`, with 3.2 kept as
intent and relabelled. Same convention and same reasoning as section 4's amendment.

### 3.1 The collector that exists
**A conservative, non-moving, mark-and-sweep collector, hand-written** in
`crates/nova-runtime/src/gc.rs`. Not bdwgc: the workspace depends on no Boehm crate, and
no third-party collector of any kind.

It is conservative **because it has to be**. Neither Nova codegen backend emits stack
maps or per-slot type information, so the collector cannot know where roots or heap
pointers precisely live. Any machine word whose value falls inside a live allocation
keeps that allocation alive. That retains a little garbage — an integer that happens to
look like a pointer — but never frees a reachable object.

Roots come from exactly three places:

- **Callee-saved registers**, flushed onto the stack by a `setjmp` shim in `gc_stack.c`.
  Caller-saved registers hold no live root at a call boundary, so they need no handling.
- **The stack**, scanned from the current frame to the thread's base.
- **Explicitly registered roots**, for objects reachable from neither. A suspended async
  task's state is owned by the Rust executor while parked — on no Nova stack and in no
  register — so it is pinned by address instead.

Marking is **range-based**, so an interior pointer (an array-element address held
transiently, say) keeps its containing object alive. Objects allocated with `scan =
false` — string byte buffers — are leaves and are never traced.

**A sweep really frees, so an address is not a durable identity for an object.** Unmarked
memory goes back to the system allocator rather than into an arena this module keeps, so
a later allocation can hand the same address out for something wholly unrelated. Nothing
may key a persistent table on an object's address.

### 3.2 MMTk — NOT BUILT, an aspiration
Modular, precise, generational; better latency and throughput. It is recorded here as
intent, not as specification.

Its blocker was stated correctly when this section was first written and is now confirmed
from the other side: MMTk needs precise stack maps from codegen, and 3.1 is conservative
*precisely because* neither backend emits them. So this is one change, not two — the
collector cannot become precise before codegen does.

### 3.3 GC interface (compiler-facing)
This section previously declared `nova_gc_alloc(size, type_id)`,
`nova_gc_register_root(slot)`, `nova_gc_safepoint()` and `nova_gc_init()`. **None of the
four exists.** What exists:

```rust
// Rust-facing, in crates/nova-runtime/src/gc.rs
pub fn alloc(size: usize, scan: bool) -> *mut u8;
pub fn add_root(ptr: *mut u8);
pub fn remove_root(ptr: *mut u8);

// C-facing, for the root-scanning shim
pub extern "C" fn nova_gc_scan_range(lo: *const c_void, hi: *const c_void);
extern "C" { fn nova_gc_collect_roots(stack_base: *mut c_void); }  // gc_stack.c
```

Three differences are substantive rather than cosmetic:

- **`scan: bool`, not `type_id: u32`.** The allocator is told whether an object must be
  traced or is a leaf. It is given no type identity, and a conservative collector has no
  use for one.
- **Roots are pinned by address, not by slot.** `add_root` takes the object pointer, not
  the address of a variable holding it.
- **There is no safepoint, and the compiler emits no safepoint calls.** This section
  previously stated that the compiler emits `nova_gc_safepoint()` at loop back-edges and
  function entries. It does not, and no such function exists. **Collection is triggered
  from `alloc`**, once allocation since the last cycle crosses a growth threshold — or on
  every allocation under `NOVA_GC_STRESS`, which exists to shake out root-scanning bugs.
  A program that allocates nothing never collects.

### 3.4 Finalizers
`Drop` → finalizer at allocation is **NOT BUILT**, and cannot be while `Drop` is
unimplemented — it is described in `12-TYPESYSTEM.md`, `14-CODEGEN.md` and here, and
implemented nowhere, with no `trait Drop` in `std/core`.

There *is* a per-object notification hook, and it is worth naming precisely because its
shape is the reason it cannot stand in for one. The sweep calls
`task::forget_freed_state(addr)` for **every** object it frees. That hook receives the
freed object's **own address and nothing else** — never a field value read out of it. So
a dying handle notifies with an address, which tells a table keyed on anything else
(a file descriptor, say) nothing at all.

This is exactly why `docs/adr/0012-file-descriptor-lifecycle.md` chose an explicit,
idempotent `close` for `File` over a collector-based backstop, and why
`docs/adr/0017-std-sync-channel-shape.md` chose explicit `close` for a channel. Any
future finalizer design needs the *language* feature, not this hook.

---

## 4. Async Runtime

**AMENDED 2026-08-22 (branch `spec-runtime-async-truth`): this section previously
specified a Tokio-backed, work-stealing, multi-threaded runtime with structured
cancellation, and every subsection of it was false.** Nothing here was ever built that
way. It is rewritten below to describe the runtime that exists, with the one genuinely
unbuilt item (§4.4) relabelled as an open gap rather than deleted, since it remains
intent. `20-STDLIB.md` is the only other file under `nova-spec/` carrying dated
amendments; this is the first in this file, and the convention is borrowed deliberately
— silently rewriting what a specification specified is worse than recording that it
changed. See `docs/adr/0017-std-sync-channel-shape.md`, which routed this correction
here.

### 4.1 Executor
A **single-threaded cooperative executor**, hand-written in
`crates/nova-runtime/src/task.rs` — whose own first line reads "A single-threaded
cooperative executor." There is no Tokio anywhere in the workspace and no thread pool:
tasks run to their next suspension point on the calling thread, and the ready queue is
FIFO round-robin (`task.rs`, the `QUEUE` doc comment), so a task re-queued by
`yield_now` waits behind every task already waiting.

- `block_on<T>(fut: Future<T>) -> T` is the entry point from a synchronous `main`.
- Calling `block_on` from inside an `async fn` **ends the process with a diagnostic**
  rather than unwinding out of the runtime through a generated frame.
- Cooperation is real rather than nominal: a task that never reaches a suspension point
  is unpreemptable, and no watchdog can fire while it spins.

### 4.2 Future Type
A Nova `Future<T>` compiles to a state machine reached through one **frozen** C ABI:

```rust
pub type PollFn = unsafe extern "C-unwind" fn(state: *mut u8, task_ctx: *mut u8) -> i64;
pub const POLL_PENDING: i64 = 0;
pub const POLL_READY: i64 = 1;
```

- `task_ctx` is **always null** at every call site in the runtime. It exists for a waker
  that has never been needed: readiness is discovered by re-polling, not by notification.
- **No panic may cross a poll boundary.** The `-unwind` in the ABI is what makes an
  escaping panic an abort rather than undefined behaviour — it is not permission to
  unwind. A Cranelift- or LLVM-emitted frame has no landing pads, so anything reachable
  from a generated call site must abort instead.
- This ABI is frozen. Changing the signature, the two status codes, or the null `task_ctx`
  breaks every generated poll function at once.

### 4.3 Task Spawning
`spawn`, `join` and `yield_now` are **free functions and methods**, not a `task.` module
path — Nova has no module-qualified call syntax.

```nova
async fn work() -> Int { 42 }

async fn main() {
    let handle = spawn(work())
    let result = handle.join().await
    println("got ${result}")
}
```

- `spawn<T>(fut: Future<T>) -> JoinHandle<T>` (`std/task/lib.nova`).
- `JoinHandle::join(self) -> T` is `async`; the handle is not itself awaitable.
- `yield_now()` re-queues the current task behind everything already runnable.

### 4.4 Cancellation — NOT BUILT, an open gap
This subsection previously specified drop-cancels-child, cooperative cancellation and a
`task.cancel()` method. **None of it exists**, and it is recorded here as intent rather
than as specification.

`docs/adr/0009-async-execution-model.md` files "No cancellation" under its *residual
gaps* and names a future `JoinHandle` drop or cancellation as the natural fix point — an
open gap, not a foreclosed decision. What ADR 0009 does settle is narrower and concerns
`timeout<T>`: because the poll ABI has no cancellation hook, `timeout` **abandons** its
inner future rather than cancelling it. An abandoned future is simply never polled again.

Two things block the design as written, and both are structural rather than incidental:

- **`Drop` is unimplemented.** It is described in `12-TYPESYSTEM.md`, this file's §3.4 and
  `14-CODEGEN.md`, and implemented nowhere, so "dropping a handle" is not an event the
  language can observe.
- **The frozen poll ABI has no interrupt hook** (§4.2). There is no way to stop a task
  mid-flight, only to stop polling it.

### 4.5 Channels
A **bounded** channel, written entirely in Nova over a private ring buffer, with no
runtime support and no Tokio (`std/sync/lib.nova`; see
`docs/adr/0017-std-sync-channel-shape.md`).

```nova
async fn main() {
    let ch: Channel<Int> = channel(2)
    let mut tx = ch.sender()
    let mut rx = ch.receiver()

    spawn(produce(tx))
    let first = rx.recv().await
}
```

- `channel<T>(buffer: Int) -> Channel<T>`, then `ch.sender()` and `ch.receiver()`. The
  annotation on `ch` is **required**: `T` appears only in the return type and Nova has no
  turbofish, so `channel(2)` alone gives the checker no way to name it.
- `Sender::send(v) -> Bool` is `async` and returns `false` only when the channel is
  closed; `try_send` returns `false` when it is full **or** closed.
- `Receiver::recv() -> Option<T>` is `async`. `None` means closed **and** drained, never
  merely empty — it is the only signal a consumer loop can terminate on, which is why
  `close` must be explicit, `Drop` being unimplemented.
- Contention is yield-and-retry rather than parking, so a waiter stays *runnable* and
  `report_deadlock` cannot see it.

---

## 5. Panic Handling

**AMENDED 2026-08-22 (branch `spec-runtime-async-truth`): this section contradicted both
the tree and itself.** It said a panic aborts the current task "not whole process by
default" and propagates to an `await` site as `Err`, three lines above a code block
calling `std::process::abort()`. Corrected against `crates/nova-runtime/src/lib.rs` and
`task.rs`.

**A panic ends the process. There is no unwinding, and no per-task recovery.**

```rust
// crates/nova-runtime/src/lib.rs
pub unsafe extern "C" fn nova_rt_panic_str(s: *const NovaStr) -> ! {
    let msg = if s.is_null() { "" } else { as_str(s) };
    eprintln!("nova: panic: {msg}");
    std::process::abort();
}
```

- **No `Err` at the `await` site.** A panic does not propagate anywhere; the process is
  gone. This is not a temporary simplification — it follows from 4.2: a generated poll
  frame has no landing pads, so nothing reachable from one may unwind.
- **No stack unwinding on the synchronous path either.** `abort`, not Rust's unwinding
  panic infrastructure.
- **There is no `--panic=abort` build flag**, and the workspace sets no `panic` profile
  key. The flag would be moot regardless: aborting is already the only behaviour.
- The internal counterpart is `task::abort_with(msg)`, used by runtime intrinsics that
  must reject an argument — negative indices, out-of-range bytes, a doubly-borrowed
  handle table. It shares the `nova: panic:` prefix and, like `nova_rt_panic_str`, does
  not unwind.

The one thing this section had right is the observable output: `nova: panic: <message>` on
stderr, then an abort exit.

## 6. FFI

**NOT BUILT — this section is a design, not a description (noted 2026-08-22, branch
`spec-runtime-async-truth`).** 6.3 labels its own phase; 6.1 and 6.2 did not, so a reader
could not tell. Measured: there is no `nova.toml` anywhere in the repo, no `@`-attribute
syntax (`@c_import`/`@c_export`) in any `.nova` file, no `unsafe` block and no `extern fn`
in any `.nova` file, and `nova build` has no `--crate-type` flag. Nothing below is
reachable today. It is left as written because it is intent, and unlike 3.3 and 7 it does
not misdescribe a mechanism that exists.


### 6.1 C FFI (calling C from Nova)
```nova
@c_import("openssl/sha.h")
extern fn SHA256(data: *u8, len: usize, out: *u8) -> *u8

unsafe {
    SHA256(buf.ptr, buf.len, out.ptr)
}
```

Compiler generates LLVM extern declarations. Linker links against C library specified in `nova.toml`:
```toml
[build.link]
c-libs = ["ssl", "crypto"]
```

### 6.2 C ABI (calling Nova from C)
```nova
@c_export
fn add(a: Int, b: Int) -> Int { a + b }
```

Generates `extern "C"` symbol. Build with `nova build --crate-type=cdylib` produces `.so`/`.dylib`/`.dll`.

### 6.3 Rust Embedding
A separate crate `nova-embed` (Phase 6) lets Rust apps embed the Nova runtime + interpret/JIT Nova source.

---

## 7. Stdlib Runtime Hooks

**AMENDED 2026-08-22 (branch `spec-runtime-async-truth`): the mechanism this section
described does not exist and was deliberately rejected, and four of its six example
functions do not exist either.** This is the section a contributor reads to add a runtime
hook, so being wrong here misdirects the exact task it exists to guide — which is why it
is corrected rather than labelled.

**There is no `extern "nova-rt"`.** No `.nova` file in the repo contains such a
declaration, and `std/core/lib.nova` records why in its own words: `nova_`-prefixed extern
symbols are reserved by the compiler, "so a user-visible `extern` was not an option
either."

### How a runtime hook actually works
A runtime-backed operation is a **compiler-known builtin**, not a declared external
symbol. Adding one means, at minimum:

1. A variant on the `Builtin` enum in `crates/nova-resolver/src/lib.rs`, plus its entry in
   the name table that makes it resolvable from Nova source (`STD_ONLY` for the
   std-only ones).
2. A signature in `crates/nova-typeck/src/check.rs`.
3. Lowering, so MIR and each codegen backend emit a call.
4. The Rust implementation, `#[no_mangle] pub unsafe extern "C" fn nova_rt_*`, in
   `crates/nova-runtime/src/`.
5. **A line in `symbols()`** (`crates/nova-runtime/src/lib.rs`), which maps the name to
   the function pointer for the JIT.

Most of those sites are compiler-forced: an omission fails to build, because the
`match`es over `Builtin` are exhaustive. **`symbols()` is the exception** — leaving a
function out of it compiles cleanly and fails at JIT link time instead. That is why
`every_rt_func_symbol_is_registered_with_the_jit`
(`crates/nova-codegen-cranelift/src/lib.rs`) exists: it is the guard for the one seam the
compiler cannot enforce. Count the forced sites yourself when you touch this — the number
has changed as the compiler has grown, and a stale count here would be worse than none.

**AMENDED 2026-08-23 (branch `std-json`): `symbols()` is not the only unforced site, and
counting the forced ones needs `--all-targets`.** Both measured while adding
`str_to_float`, that increment's one intrinsic, which touched 12 sites of which 7 were
compiler-forced — 12 under the counting rule ADR 0018 §3 states, which is what makes the
figure reproducible; a seam count without its rule is not.

1. **`STD_ONLY` membership is unforced too.** Omitting the array element *and* its length
   together compiles the whole workspace clean, test targets included; only a
   length/element mismatch is checked. Its consequence is loud rather than latent, though
   — every `nova` invocation then fails with a Nova-level `error[E0001]: cannot find
   function` as soon as a std module calls the builtin — so `symbols()` remains the only
   site whose omission survives *every* compiler in the pipeline, Rust's and Nova's, all
   the way to JIT link time. That is the accurate form of the claim above, which the
   paragraph overstated by saying "the exception".
2. **A plain `cargo check --workspace` undercounts the forced sites.** One of them is a
   description table inside `nova-typeck`'s `#[cfg(test)] mod tests`, so without
   `--all-targets` the count comes out one short *and the build reports success*. Nor do
   they all surface in one pass: the resolver's own name table fires alone first, because
   cargo cannot compile the downstream crates until the resolver builds.

See `docs/adr/0018-std-json-scope-and-build-order.md` §3. The advice above still stands —
count them yourself; these two facts are about *how* to count, not a number to reuse.

**AMENDED 2026-08-28 (branch `seeded-mix64`): another intrinsic went through
these seams, and what it settled is about the seams rather than about a number.**
`int_hash_seed`, backed by `nova_rt_int_hash_seed`, was added for `std/core`'s
`Int`, `Bool` and `Char` hashing. This section states no roster of intrinsics and
no `STD_ONLY` length, deliberately, and neither was added here — `symbols()` and
`Builtin::STD_ONLY` in `crates/nova-resolver/src/lib.rs` are the live lists, as
the paragraphs above say.

1. **`Builtin::ALL` and `RtFunc::ALL` are not sites.** Both variant lists are
   generated by a `macro_rules!` — `builtins!` in `crates/nova-resolver/src/lib.rs`
   and `rt_funcs!` in `crates/nova-mir/src/lib.rs` — and each derives its `ALL`
   array *and that array's length* from the same identifier list that declares
   the enum, so a variant cannot exist without appearing in `ALL` and there is no
   separate `ALL` entry to forget. `STD_ONLY` is a different kind of thing: a
   hand-written array with a hand-written length annotation, which is why the
   unforced-site paragraph above is about it rather than about `ALL`.
2. **The `E0001` consequence above turns on its "as soon as a std module calls
   the builtin" clause, and this increment met the window where that clause does
   not hold.** The intrinsic landed in one commit with no `std` caller, and a
   later commit added the `Hash` impls that call it. Inside that window,
   omitting the `STD_ONLY` element and its length together has nothing left to
   surface through: the paragraph above records that the pair compiles the whole
   workspace clean, test targets included, and with no Nova caller there is no
   name for a `nova` invocation to fail to resolve. That is an inference from
   the facts above rather than a measurement — nobody ran the omission in that
   state — so read the clause as a condition to check rather than as scenery.

### The examples this section used to give
`nova_rt_println` and `nova_rt_eprintln` exist, though their parameter is a
`*const NovaStr` rather than the `*str` written here. **`nova_rt_read_file`,
`nova_rt_tcp_connect`, `nova_rt_http_serve` and `nova_rt_json_parse` do not exist in any
form** — `std/fs` and `std/net` reach the runtime under different names and shapes, and
`std/http` and `std/json` are unstarted (see `00-MASTER-SPEC.md` section 3, positions
10-11). Read `symbols()` for the live list rather than trusting an example here.

**AMENDED 2026-08-23 (branch `std-json`): `std/json` is no longer unstarted; `std/http`
still is.** `std/json` shipped at position 11, ahead of position 10, which remains
unstarted and unblocked (see `docs/adr/0018-std-json-scope-and-build-order.md`). The
paragraph's point survives intact for the symbol it names: **`nova_rt_json_parse` still
does not exist in any form** — grepped, along with `nova_rt_http_serve`,
`nova_rt_read_file` and `nova_rt_tcp_connect`, none of which appear anywhere in `crates/`
or `std/`. `std/json` is Nova source over exactly one new runtime entry point,
`nova_rt_str_to_float`, and adds no JSON-specific one. So this section's advice is
unchanged: read `symbols()`.

**AMENDED 2026-09-01 (branch `std-http-parsing`): `std/http` is no longer
unstarted either, and `nova_rt_http_serve` still does not exist — under that
name or any other shape resembling it.** The server half ships: request-head
parsing over one intrinsic, `nova_rt_http_parse_request`
(`crates/nova-runtime/src/http.rs`), plus response serialisation over none at
all (`docs/adr/0019-offset-table-intrinsic-boundary.md`). That intrinsic
parses a request head into an offset table; it does not "serve" anything —
no accept loop, no connection lifecycle, no handler dispatch reaches the
runtime at all, all of that being Nova-side `std/http` and `std/net` code
calling ordinary `read`/`write`. So the earlier paragraph's own point about
`nova_rt_http_serve` is, if anything, stronger now than when it was
speculative: grepped again, it still does not exist in any form, and the
shape this increment shipped instead makes clear it never will under that
name. `std/json` is unaffected by this note; see the amendment above for its
own status.

Each runtime function does have a stable C ABI signature and is documented at its
definition, as this section originally said.

## 8. WASM Runtime (Phase 4)

For browser target:
- No host executor (use the browser event loop via wasm-bindgen)
- No host collector (use a simple GC OR integrate the WASM GC proposal when stable)
- Phase 4 decision: **start with reference counting** for WASM target (simpler, smaller); revisit later
- DOM access via auto-generated bindings from web-sys

---

## 9. Build Config Impact

The runtime is large. Minimize:
- `--release` builds with LTO, strip symbols
- Tree-shake unused std features (link-time + dead-code elim)
- `nova build --minimal` excludes the executor and async runtime hooks if no async used

Target binary sizes:
- Hello world: < 5 MB (current Rust+Tokio is ~3 MB; Nova adds GC overhead)
- WASM hello world: < 30 KB gzipped

---

## 10. Testing

- Unit tests in `crates/nova-runtime/src/`
- Integration tests via running compiled Nova programs
- Stress tests: spawn 10k tasks, verify no leaks
- Benchmark: HTTP server req/sec, JSON parse speed, allocation throughput

The first two exist. **The last two do not** (noted 2026-08-22, branch
`spec-runtime-async-truth`): there is no stress or benchmark harness in the repo, and two
of the three benchmark subjects are unstarted modules. `NOVA_GC_STRESS` (3.3) is the one
stress mechanism that does exist, and it targets root scanning rather than task counts.

**AMENDED 2026-09-01 (branch `std-http-parsing`): neither benchmark subject
named above is an unstarted module any more, and the harness gap is the part
that still holds.** `std/json` shipped on 2026-08-23
(`docs/adr/0018-std-json-scope-and-build-order.md`) and `std/http`'s server
half ships on this branch (`docs/adr/0019-offset-table-intrinsic-boundary.md`),
so "two of the three benchmark subjects are unstarted modules" is no longer
true of any of the three — allocation throughput was always benchmarkable,
being Phase 1 infrastructure. What the 2026-08-22 note actually established,
and what is still true, is the harness gap: there is still no stress or
benchmark harness in this repo, `examples/05-json-api` still does not exist,
and `docs/benchmarks/` still does not exist. A module existing is not the
same claim as a benchmark of it existing, and this correction narrows to
exactly the first.

**AMENDED 2026-09-03 (branch `phase-2-gate-benchmark`): the benchmark half
of the harness-gap sentence above is now false; the stress half is not.** A
benchmark harness exists now — `crates/nova-bench-http`, a dependency-free
load generator, plus the procedure and dated observation in
`docs/benchmarks/` (`README.md` and `http-fixed-response.md`) — so "there is
still no stress or benchmark harness in this repo" no longer holds of the
benchmark half. No stress harness of the kind the original roster named
("spawn 10k tasks, verify no leaks") exists; `NOVA_GC_STRESS` (3.3) remains
the one stress mechanism this tree has, and it targets root scanning rather
than task counts, exactly as recorded above. `docs/benchmarks/` now exists,
which falsifies the sentence's third clause on its own; `examples/05-json-api`
still does not, so the sentence's second clause still holds.
`docs/benchmarks/http-fixed-response.md` records one measured figure,
11,940.0 req/sec against `std/http`'s read-and-parse path — excluding
response serialisation, and taken on the Cranelift backend rather than the
optimising LLVM one, which cannot run on that host — and it numerically
clears the absolute 10k+ criterion in `00-MASTER-SPEC.md` §3's Phase 2 gate,
on one host and one run. See `docs/benchmarks/http-fixed-response.md` and
`docs/benchmarks/README.md` for what that figure does and does not cover. No
claim is made that the gate itself is passed, since `examples/05-json-api`
remains unwritten and the gate's other criterion, a ratio against Bun
(`60-EXAMPLES.md` §5), is entirely unmeasured.
