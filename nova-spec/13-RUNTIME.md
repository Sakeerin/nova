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

- `panic!("message")` aborts the current task (not whole process by default)
- For async tasks: panic propagates to `await` site as `Err`
- For sync: unwind stack (use Rust's panic infrastructure)
- Build flag `--panic=abort` available for smaller binaries

```rust
#[no_mangle]
pub extern "C" fn nova_panic(msg: *const u8, len: usize) -> ! {
    let msg = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(msg, len)) };
    eprintln!("nova: panic: {}", msg);
    print_backtrace();
    std::process::abort();
}
```

---

## 6. FFI

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

These functions are implemented in Rust runtime, called by std/* Nova code via `extern "nova-rt"`:

```
nova_rt_println(s: *str)
nova_rt_eprintln(s: *str)
nova_rt_read_file(path: *str, out: *Result<Vec<u8>, IoError>)
nova_rt_tcp_connect(addr: *str) -> *TcpStream
nova_rt_http_serve(handler_fn: *fn, addr: *str)
nova_rt_json_parse(s: *str) -> *JsonValue
... etc
```

Each runtime function:
- Has a stable C ABI signature
- Documented in `crates/nova-runtime/src/lib.rs`
- Has corresponding `extern "nova-rt"` declaration in `std/`

---

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
