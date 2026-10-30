# 14 — Codegen Specification

> Crates: `nova-codegen-llvm`, `nova-codegen-cranelift`, `nova-codegen-wasm`
> Phase: 1 (Cranelift first, then LLVM), 4 (WASM)

---

## 1. IR Levels

```
TypedAst → HIR → MIR → CodegenIR
```

**HIR** (`nova-hir`):
- Desugared: `if-else` if-let, `for` → `while`, string interp → fn calls
- Names fully resolved
- Types attached to every expression

**MIR** (`nova-mir`):
- 3-address code, basic blocks, control-flow graph
- Explicit allocations, loads, stores
- Pattern matches lowered to switch/branch
- Borrows and moves explicit
- Drop points inserted

**CodegenIR**: backend-specific (LLVM IR, Cranelift IR, or WASM bytecode)

---

## 2. Backends

### 2.1 Cranelift (debug builds — fast compile)
- Crate: `cranelift`, `cranelift-jit`, `cranelift-object`
- Target: `nova run` (in-memory JIT) or `nova build --debug` (object file)
- Optimization: minimal (focus on compile speed)
- Use case: dev loop

### 2.2 LLVM (release builds — fast runtime)
- Crate: `inkwell` (safe LLVM 17 bindings)
- Target: `nova build --release`
- Optimization: O2 default, O3 with `-O3` flag, LTO with `--lto`
- Use case: production binaries

**Amended 2026-10-06 (branch `phase-2-closeout`): there is no `inkwell`.**
`crates/nova-codegen-llvm` emits LLVM IR as text, and `nova-driver`'s
`link.rs` passes it to `clang`, or to `llc` when `clang` is missing. That holds
for §7's sketch and §9.1's `inkwell::debug_info` too. End to end, only hello
world is built and run in release, by a test that passes without running
anything when `clang` is not on `PATH`. Parity with the Cranelift backend is
deferred: `docs/adr/0025-phase-2-boundary.md`.

**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the
LLVM backend's parity is now tested: every golden program of
`tests/runtime/` but the nine an exclusion table names is built with
`--release`, through Ubuntu's `clang-18`
18.1.3, pinned on CI's `ubuntu-24.04` leg, run, and compared with its
`.stdout`, in `release_golden_suite`
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§3.2; ADR 0048, which also makes textual `.ll` through `clang` the backend
of Phase 5's compiler). The hello-world test stays.

### 2.3 WASM (browser target)
- Crates: `wasm-encoder` (write modules), `walrus` (manipulate)
- Target: `nova build --target wasm` and `nova bundle`
- Output: `.wasm` file + JS shim
- Use case: frontend

**Amended 2026-10-14 (branch `claude/loving-archimedes-frtf5q`):**
`crates/nova-codegen-wasm` lowers every MIR statement to one module with
`wasm-encoder` 0.243.0, the last release that declares a minimum Rust under the
workspace's 1.78. There is no `walrus`: MIR already holds only what `main`
reaches. The runtime is `std/rt`, written in Nova on memory builtins, and the
shim provides printing, `Float` text and case mapping.
`nova build --target wasm` and `nova bundle` write `dist/`
(`docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`; ADR
0039).

---

## 3. Calling Convention

- **Native:** C ABI (System V on Linux/Mac, Win64 on Windows)
- **WASM:** WASM standard
- **Closures:** fat pointer `{ fn_ptr, env_ptr }`
- **Async fn:** state machine struct with `poll(cx) -> Poll<T>`

---

## 4. ABI Decisions

| Type | Native ABI | WASM |
|---|---|---|
| `Int` (i64) | i64 register | i64 |
| `Float` | f64 register | f64 |
| `Bool` | i8 | i32 |
| `String` | (ptr, len) two-word | (ptr i32, len i32) |
| Tuple | flatten if small (<=2 words), else by-ref | flatten |
| Record | by reference (heap pointer) | by reference |
| Sum type | tagged: by reference | by reference |
| Closures | (fn_ptr, env_ptr) | (table_idx, env_ptr) |

**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** a tuple is passed
by reference, a heap pointer, as a record is, on both targets: it is never
flattened
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§4.1; `13-RUNTIME.md` §2.2). 5.0b's ADR 0049 records the departure.

---

## 5. Compilation Flow Per Function

```
1. Parse function body → AST
2. Type-check → TypedAst with `Ty` per node
3. Lower to HIR (desugar)
4. Lower to MIR (CFG, basic blocks)
5. Run analysis passes:
   - Liveness
   - Drop insertion
   - Move/copy detection
6. Emit codegen IR (LLVM or Cranelift)
7. Run backend optimization passes (LLVM only at release)
8. Emit object code
9. Link with runtime + std lib
```

---

## 6. Specific Lowering Rules

### 6.1 String Interpolation
```nova
"Hello, ${name}!"
```
Lowers to:
```nova
let mut __s = String::with_capacity(32)
__s.push_str("Hello, ")
__s.push_str(&name.to_string())
__s.push_str("!")
__s
```

**Amended 2026-10-01:** the lowering that ships is not the builder above.
`crates/nova-mir/src/lower.rs` lowers an interpolation of three or more
parts to one heap array of the already-stringified parts and one call to
`nova_rt_str_concat_n`, which copies each part once into one new string. A
two-part interpolation is one `nova_rt_str_concat`, and a one-part
interpolation is the part itself. Until 2026-10-01 every interpolation of
two or more parts was a chain of pairwise `nova_rt_str_concat` calls, each
copying everything built so far. See `examples/05-json-api/BENCHMARK.md`, "AMENDMENT
2026-10-01 (nary-interpolation)".

**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):** the
builder a program writes by hand is not `String::with_capacity` and
`push_str` either: it is std/strings' `StringBuilder`, a record over a
`Vec<String>` of the parts and their length in bytes
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§5.1). `push` is a `Vec` push; `to_string` copies the parts' pointers into
an array of their count, then joins them with one `nova_rt_str_join`, one
copy of the bytes. No backend and no runtime symbol changed for it.

### 6.2 Pattern Match
Compiled to decision tree (Maranget). Each leaf is a basic block. Compiler emits:
```llvm
switch i64 %tag, label %default [ i64 0, label %case0
                                  i64 1, label %case1 ]
```

**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** a
`match` is lowered to MIR by test chains, not a decision tree
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§4.6; ADR 0049). A match whose arms are all flat (`_`, a name, a literal,
or a variant whose fields are names or `_`) and unguarded lowers as before:
one `Switch` on the tag or the value (the `switch i64` above, or `i8` for a
`Bool`), or a chain of string compares. Any other match lowers wholly by
chains: the scrutinee is evaluated once into a temporary; each arm, in
order, tests its pattern left to right and depth first, a tag (`SumTag`), an
`Int`, `Char` or `Bool` value, a string (`StrEq`) or a range (two compares),
each on a value projected once with `SumField` or `RecordField`; a failed
test jumps to the next arm's chain. A sum of one variant takes no tag test.
The arm binds its names as the chain reaches them, then runs its guard,
whose false edge also goes to the next arm, then its body. An or-pattern's
alternatives are tested in turn on one value and join where they succeed,
so nothing after it is duplicated. A trap follows the last arm. No MIR
statement is new, so the backends emit compares and conditional branches.

### 6.3 Generics
Monomorphized: each unique instantiation gets its own compiled function.
- `vec.push::<Int>(1)` and `vec.push::<String>("hi")` → two distinct functions in object file
- Mangled: `_nova_vec_push_Int`, `_nova_vec_push_String`

### 6.4 Async fn
Lowers to state machine. `nova-async-lowering` pass converts:
```nova
async fn foo() -> Int {
    let x = bar().await
    x + 1
}
```
into a struct + `poll()` impl, just like Rust's async.

### 6.5 Closures
```nova
let f = |x| x + offset
```
- Captures by reference unless explicitly `move`
- Compiled to anonymous record with `Fn` impl
- Closure call lowers to indirect function call through fat pointer

---

## 7. LLVM Pipeline Sketch

```rust
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;

pub fn codegen_module(mir: &Mir) -> Module {
    let ctx = Context::create();
    let module = ctx.create_module(&mir.name);
    let builder = ctx.create_builder();

    // 1. Emit type metadata
    emit_type_table(&ctx, &module, &mir.types);

    // 2. Forward-declare all functions
    for func in &mir.functions {
        declare_fn(&ctx, &module, func);
    }

    // 3. Define each function
    for func in &mir.functions {
        define_fn(&ctx, &module, &builder, func);
    }

    // 4. Emit GC roots metadata
    emit_gc_roots(&module);

    // 5. Verify
    module.verify().expect("LLVM module verification failed");

    module
}
```

---

## 8. WASM Pipeline (Phase 4)

```rust
use wasm_encoder::*;

pub fn codegen_wasm(mir: &Mir) -> Vec<u8> {
    let mut module = Module::new();

    // Section: types
    let mut types = TypeSection::new();
    // ... add all function types
    module.section(&types);

    // Section: imports (JS host functions)
    let mut imports = ImportSection::new();
    imports.import("js", "log", EntityType::Function(0));
    module.section(&imports);

    // Section: functions, code, exports, memory, table, ...

    module.finish()
}
```

Plus a `nova-bundler` step that:
1. Generates JS shim that loads the wasm
2. Stubs out import functions (DOM, fetch, etc.)
3. Tree-shakes
4. Outputs `app.wasm` + `app.js`

**Amended 2026-10-14 (branch `claude/loving-archimedes-frtf5q`):** as built in
4.0, the backend writes the shim as well, from the host functions the module
imports, and `nova-driver` writes `dist/`. Values map as `I64` → `i64`, `F64` →
`f64`, `I8` → `i32` and `Ptr` → `i32`; slots stay 8 bytes; control flow is
structured by Ramsey's "Beyond Relooper", with a dispatch loop for an
irreducible graph; function values are `funcref` table indices. Tree shaking is
monomorphization's. `nova-bundler` is still empty (ADR 0039).

**Amended 2026-10-15 (branch `claude/loving-archimedes-frtf5q`):** in 4.1 the
backend builds the yield future itself, a builder and a poll function of type
`(state, ctx) -> status` in the table, and lowers two more WASM-only
builtins: `future_poll`, a `call_indirect` with a null context, and
`host_abort`, an import of the shim's `abort`. A module exports
`nova_alloc_leaf`, and, with an `async fn main`, `nova_turn` and
`nova_gc_switches`. `std/rt`'s state words are `8..128`, and the data segment
starts at 128
(`docs/superpowers/specs/2026-10-15-phase-4-1-the-language-on-wasm-design.md`
§4.3, §6.1; ADR 0040).

**Amended 2026-10-16 (branch `claude/loving-archimedes-frtf5q`):** in 4.2 the
backend imports std's `extern "js"` functions from `env`, after the
runtime's host functions and before `abort`, and lowers two more WASM-only
builtins: `closure_word`, a closure's address as a word, and
`closure_call`, a `call_indirect` of type `(env, i64)`. A module that
registers a closure JS may call exports `nova_call`, one that waits on a
token `nova_wake`, and either exports `nova_turn` and `nova_gc_switches`
(`docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
§3.1, §5, §7; ADR 0041).

---

## 9. Debugging

### 9.1 DWARF
- LLVM emits DWARF 4
- Source maps from MIR locations → original `.nova` line/col
- Set up in `inkwell::debug_info`

### 9.2 WASM Source Maps
- Emit `sourceMappingURL` comment in JS shim
- Use `wasm-tools strip` then attach source map

---

## 10. Performance Targets

### 10.1 Compile Speed (per file, ~500 lines)
- Cranelift debug: < 100ms per file
- LLVM release: < 1s per file (LLVM is the bottleneck — accept it)
- Incremental: < 50ms for cached file

### 10.2 Runtime Speed
- Hello world: native binary cold start < 2ms
- Fibonacci(40): comparable to Rust release (within 20%)
- HTTP echo server: > 150k req/sec on M2-class hardware (Bun is ~120k)

### 10.3 Binary Size
- Stripped release hello world: < 5 MB
- With UPX: < 1 MB
- WASM hello world (gzipped): < 30 KB

---

## 11. Linking

- Native: use system linker (`cc`, `clang`, `link.exe`) via `cc-rs`
- Static linking by default (no shared libnova needed)
- WASM: no linker needed, single module

---

## 12. Tests

1. **End-to-end:** `nova run examples/01-hello-world/main.nova` produces "Hello, World!"
2. **Codegen snapshot:** for tiny inputs, snapshot the LLVM IR or Cranelift IR
3. **Differential:** Cranelift output and LLVM output produce identical results for deterministic programs
4. **Performance regression:** Criterion benchmarks tracked over time
