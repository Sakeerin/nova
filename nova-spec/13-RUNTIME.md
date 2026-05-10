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

This avoids re-implementing Tokio, Hyper, etc. — leverage the Rust ecosystem.

```
+-----------------------------------------+
|  Compiled Nova Code (LLVM-generated)    |
|    calls into ↓                         |
+-----------------------------------------+
|  nova-runtime (Rust)                    |
|    - GC (mmtk or bdwgc)                 |
|    - Tokio executor                     |
|    - Hyper HTTP                         |
|    - Ring crypto                        |
|    - Allocator                          |
|    - Panic infrastructure               |
+-----------------------------------------+
|  OS (libc)                              |
+-----------------------------------------+
```

---

## 2. Memory Layout

### 2.1 Object Header
Every heap-allocated Nova object has a header:

```rust
#[repr(C)]
pub struct ObjectHeader {
    pub type_id: u32,       // index into type metadata table
    pub flags: u32,         // GC flags (mark, generation, etc.)
    // followed by fields
}
```

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

### 3.1 Phase 1 (MVP): bdwgc (Boehm)
- Conservative, mark-and-sweep
- Easy integration: just call `GC_malloc` instead of `malloc`
- Trade-off: false retention (conservative scanning), pause times not great
- Pro: works immediately, well-tested

### 3.2 Phase 2+: MMTk
- Modular, precise, generational
- Enables better latency, throughput
- More work to integrate (need precise stack maps from codegen)

### 3.3 GC Interface (compiler-facing)
```rust
extern "C" {
    pub fn nova_gc_alloc(size: usize, type_id: u32) -> *mut ObjectHeader;
    pub fn nova_gc_register_root(slot: *mut *mut ObjectHeader);
    pub fn nova_gc_safepoint();
    pub fn nova_gc_init();
}
```

Compiler emits `nova_gc_safepoint()` at loop back-edges and function entries.

### 3.4 Finalizers
- `Drop` trait → finalizer registered at allocation
- Best-effort, not guaranteed (matches typical GC semantics)

---

## 4. Async Runtime

### 4.1 Executor
- Wrap **Tokio** as the runtime
- `nova_runtime_block_on(future)` for `main` if async
- Standard work-stealing thread pool, sized to CPU count

### 4.2 Future Type
A Nova `Future<T>` compiles to a state machine struct (like Rust's async). At runtime:
```rust
trait NovaFuture {
    fn poll(&mut self, cx: &mut Context) -> Poll<NovaValue>;
}
```

### 4.3 Task Spawning
```nova
let handle = task.spawn(async {
    // ...
})
let result = handle.await
```

### 4.4 Cancellation
- Structured concurrency: dropping handle cancels child task
- Cooperative cancellation: tasks must hit `.await` for cancel to fire
- `task.cancel()` explicit method

### 4.5 Channels
```nova
let (tx, rx) = sync.channel::<Int>(buffer: 100)
task.spawn(async { tx.send(42).await })
let v = rx.recv().await
```

Backed by Tokio's `mpsc::channel`.

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
- No Tokio (use browser event loop via wasm-bindgen)
- No bdwgc (use a simple GC OR integrate WASM GC proposal when stable)
- Phase 4 decision: **start with reference counting** for WASM target (simpler, smaller); revisit later
- DOM access via auto-generated bindings from web-sys

---

## 9. Build Config Impact

The runtime is large. Minimize:
- `--release` builds with LTO, strip symbols
- Tree-shake unused std features (link-time + dead-code elim)
- `nova build --minimal` excludes Tokio if no async used

Target binary sizes:
- Hello world: < 5 MB (current Rust+Tokio is ~3 MB; Nova adds GC overhead)
- WASM hello world: < 30 KB gzipped

---

## 10. Testing

- Unit tests in `crates/nova-runtime/src/`
- Integration tests via running compiled Nova programs
- Stress tests: spawn 10k tasks, verify no leaks
- Benchmark: HTTP server req/sec, JSON parse speed, allocation throughput
