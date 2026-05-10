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

### 2.3 WASM (browser target)
- Crates: `wasm-encoder` (write modules), `walrus` (manipulate)
- Target: `nova build --target wasm` and `nova bundle`
- Output: `.wasm` file + JS shim
- Use case: frontend

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

### 6.2 Pattern Match
Compiled to decision tree (Maranget). Each leaf is a basic block. Compiler emits:
```llvm
switch i32 %tag, label %default [ i32 0, label %case0
                                  i32 1, label %case1 ]
```

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
