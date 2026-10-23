//! The WASM backend: one MIR module to one WASM module (spec
//! `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
//! §4). See ARCHITECTURE.md for the pipeline.
//!
//! Values map as `I64` → `i64`, `F64` → `f64`, `I8` → `i32` (0 or 1),
//! `Ptr` → `i32`, and `Unit` to no value; each non-`Unit` temp is a local.
//! Blocks are structured by [`structure`]'s analysis, or, for an
//! irreducible graph, run by a dispatch loop. Runtime functions are of four
//! kinds on WASM ([`nova_mir::WasmRuntime`]): `std/rt`'s, which MIR already
//! calls directly; the shim's, imported from `env`; the one this backend
//! emits itself, the yield future (spec
//! `docs/superpowers/specs/2026-10-15-phase-4-1-the-language-on-wasm-design.md`
//! §4.3); and none.

mod shim;
mod structure;

pub use shim::{shim, INDEX_HTML, PACKAGE_JSON};

use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use nova_mir::{
    BinOp, ExternAbi, Function as MirFn, MemOp, MirTy, Module, OperandClass, RtFunc, Stmt, Temp,
    Terminator, WasmRuntime,
};
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataSection, ElementSection, Elements, EntityType,
    ExportKind, ExportSection, Function, FunctionSection, ImportSection, Instruction as I, MemArg,
    MemorySection, MemoryType, RefType, TableSection, TableType, TypeSection, ValType,
};

use structure::{Cfg, UNREACHED};

/// Memory's first size, in 64 KiB pages: 16 MB (`30-FRONTEND.md` §10).
pub const MEMORY_PAGES: u64 = 256;
/// Memory's largest size, in pages: 1 GB.
pub const MEMORY_MAX_PAGES: u64 = 16384;
/// Where the data segment starts (spec §4.1, 4.1 §6.1): `0..8` is never an
/// object, and `8..128` holds `std/rt`'s state words.
pub const DATA_BASE: u32 = 128;

/// The most locals a function may have, its parameters included: V8 and
/// `wasmparser` refuse a function with more.
pub const MAX_LOCALS: usize = 50_000;

/// A limit of WebAssembly's that the program exceeds: the program's to
/// change, not a fault of the compiler.
#[derive(Debug)]
pub struct Limit(String);

impl std::fmt::Display for Limit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Limit {}

/// The compiling thread's stack: structuring recurses once per nested join
/// or loop, so a long function needs more than a default thread's.
const STACK: usize = 256 << 20;

/// What [`compile_module`] makes.
#[derive(Debug)]
pub struct Output {
    /// The module's bytes.
    pub wasm: Vec<u8>,
    /// The shim's functions it imports from `env`, by field name, in the
    /// order of the module's imports: the runtime's, then std's
    /// `extern "js"` functions, then `abort`.
    pub imports: Vec<String>,
}

/// The WASM value of a MIR class, or none for `Unit`.
fn val(ty: MirTy) -> Option<ValType> {
    match ty {
        MirTy::I64 => Some(ValType::I64),
        MirTy::F64 => Some(ValType::F64),
        MirTy::I8 | MirTy::Ptr => Some(ValType::I32),
        MirTy::Unit => None,
    }
}

fn vals(tys: &[MirTy]) -> Vec<ValType> {
    tys.iter().filter_map(|t| val(*t)).collect()
}

/// A deduplicating type section.
#[derive(Default)]
struct Types {
    section: TypeSection,
    index: HashMap<(Vec<ValType>, Vec<ValType>), u32>,
}

impl Types {
    fn of(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let key = (params, results);
        if let Some(&i) = self.index.get(&key) {
            return i;
        }
        let i = self.index.len() as u32;
        self.section
            .ty()
            .function(key.0.iter().copied(), key.1.iter().copied());
        self.index.insert(key, i);
        i
    }
}

/// A function's ABI parameters: the environment pointer when it takes one,
/// then its parameters, as MIR's temps hold them.
fn abi_params(f: &MirFn) -> &[MirTy] {
    let n = f.params as usize + usize::from(f.takes_env);
    &f.temps[..n]
}

/// What the module knows while lowering each function.
struct Layout {
    /// Each function's index, by name: imports first, then MIR's functions.
    funcs: HashMap<String, u32>,
    /// Each MIR function's return class, by name.
    rets: HashMap<String, MirTy>,
    /// Each MIR function's ABI parameter classes, by name.
    params: HashMap<String, Vec<MirTy>>,
    /// Each host function's index, by `RtFunc`.
    hosts: HashMap<RtFunc, u32>,
    /// The heap's first address ([`MemOp::HeapBase`]).
    heap_base: i64,
    /// Each string literal's header, by its text (spec §4.5).
    literals: HashMap<String, u32>,
    /// Each function value's place in the table, by the function's name.
    table: HashMap<String, u32>,
    /// `std/rt`'s allocator, when the module has one.
    alloc: Option<u32>,
    /// The type of each `call_indirect` the module makes: `(env, params...)
    /// -> ret`, interned before any function is lowered.
    indirect: HashMap<(Vec<ValType>, Vec<ValType>), u32>,
    /// The yield future's builder, when the module builds one.
    yield_future: Option<u32>,
    /// The shim's `abort`, when the module imports it.
    abort: Option<u32>,
}

/// A poll function's type on WASM: `(state, ctx) -> status` (ADR 0039).
fn poll_type() -> (Vec<ValType>, Vec<ValType>) {
    indirect_type(&[MirTy::Ptr], MirTy::I64)
}

/// A `fn(Int)` closure's type on WASM: `(env, arg)` (spec 4.2 §5.1).
fn closure_type() -> (Vec<ValType>, Vec<ValType>) {
    indirect_type(&[MirTy::I64], MirTy::Unit)
}

/// A `call_indirect`'s type: the environment pointer, then `params`.
fn indirect_type(params: &[MirTy], ret: MirTy) -> (Vec<ValType>, Vec<ValType>) {
    let mut abi = vec![ValType::I32];
    abi.extend(vals(params));
    (abi, val(ret).into_iter().collect())
}

/// The data segment: each distinct string literal's 16-byte header,
/// `{len: i64, ptr: i32, 0: i32}`, then its bytes, each part 8-aligned,
/// from [`DATA_BASE`] (spec §4.5). Returns the bytes and each header's
/// address.
fn literal_data(mir: &Module) -> (Vec<u8>, HashMap<String, u32>) {
    let mut data: Vec<u8> = Vec::new();
    let mut at: HashMap<String, u32> = HashMap::new();
    for stmt in mir
        .functions
        .iter()
        .flat_map(|f| &f.blocks)
        .flat_map(|b| &b.stmts)
    {
        let Stmt::ConstStr(_, text) = stmt else {
            continue;
        };
        if at.contains_key(text) {
            continue;
        }
        let header = DATA_BASE + data.len() as u32;
        let bytes = header + 16;
        data.extend_from_slice(&(text.len() as u64).to_le_bytes());
        data.extend_from_slice(&bytes.to_le_bytes());
        data.extend_from_slice(&[0; 4]);
        data.extend_from_slice(text.as_bytes());
        while data.len() % 8 != 0 {
            data.push(0);
        }
        at.insert(text.clone(), header);
    }
    (data, at)
}

/// Lower `mir` to a WASM module. `main` is exported, with the memory. A
/// function over a limit of WebAssembly's is a [`Limit`] error.
pub fn compile_module(mir: &Module) -> Result<Output> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("nova-codegen-wasm".to_string())
            .stack_size(STACK)
            .spawn_scoped(scope, || compile(mir))
            .map_err(|e| anyhow!("starting the compiling thread: {e}"))?
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

fn compile(mir: &Module) -> Result<Output> {
    let mut types = Types::default();
    let mut imports = ImportSection::new();
    let mut layout = Layout {
        funcs: HashMap::new(),
        rets: HashMap::new(),
        params: HashMap::new(),
        hosts: HashMap::new(),
        heap_base: i64::from(DATA_BASE),
        literals: HashMap::new(),
        table: HashMap::new(),
        alloc: None,
        indirect: HashMap::new(),
        yield_future: None,
        abort: None,
    };
    let (data, literals) = literal_data(mir);
    layout.heap_base = i64::from(DATA_BASE) + data.len() as i64;
    layout.literals = literals;

    // The host functions the program calls, in first-use order; whether it
    // builds a yield future, polls one, or aborts.
    let mut host_order: Vec<RtFunc> = Vec::new();
    let (mut yields, mut polls, mut aborts, mut calls) = (false, false, false, false);
    for f in &mir.functions {
        for b in &f.blocks {
            for s in &b.stmts {
                if let Stmt::Mem { op, .. } = s {
                    polls |= *op == MemOp::Poll;
                    aborts |= *op == MemOp::Abort;
                    calls |= *op == MemOp::ClosureCall;
                }
                if let Stmt::CallRuntime { func, .. } = s {
                    match func.on_wasm() {
                        WasmRuntime::Host(_) => {
                            if !host_order.contains(func) {
                                host_order.push(*func);
                            }
                        }
                        WasmRuntime::Builtin => yields = true,
                        WasmRuntime::Nova(name) => bail!(
                            "`{}` reached the WASM backend: MIR must call `std/rt`'s `{name}`",
                            func.symbol()
                        ),
                        WasmRuntime::Unavailable => bail!(
                            "`{}` reached the WASM backend, which has no such function",
                            func.symbol()
                        ),
                    }
                }
            }
        }
    }
    let mut import_names: Vec<String> = Vec::new();
    for (i, func) in host_order.iter().enumerate() {
        let WasmRuntime::Host(name) = func.on_wasm() else {
            unreachable!("only host functions are collected");
        };
        let (params, ret) = func.signature();
        let ty = types.of(vals(&params), val(ret).into_iter().collect());
        imports.import("env", name, EntityType::Function(ty));
        layout.hosts.insert(*func, i as u32);
        import_names.push(name.to_string());
    }
    // std's `extern "js"` functions, which MIR calls by their symbols
    // (spec 4.2 §3.1). A C function cannot reach here: the resolver refuses
    // an `extern` block of a user's on WASM.
    for ext in &mir.externs {
        if ext.abi != ExternAbi::Js {
            bail!(
                "extern \"C\" function `{}` reached the WASM backend",
                ext.symbol
            );
        }
        let ty = types.of(vals(&ext.params), val(ext.ret).into_iter().collect());
        imports.import("env", &ext.symbol, EntityType::Function(ty));
        layout
            .funcs
            .insert(ext.symbol.clone(), import_names.len() as u32);
        layout.rets.insert(ext.symbol.clone(), ext.ret);
        layout.params.insert(ext.symbol.clone(), ext.params.clone());
        import_names.push(ext.symbol.clone());
    }
    if aborts {
        let ty = types.of(vec![], vec![]);
        imports.import("env", "abort", EntityType::Function(ty));
        layout.abort = Some(import_names.len() as u32);
        import_names.push("abort".to_string());
    }
    let first = import_names.len() as u32;
    for (i, f) in mir.functions.iter().enumerate() {
        layout.funcs.insert(f.name.clone(), first + i as u32);
        layout.rets.insert(f.name.clone(), f.ret);
        layout.params.insert(f.name.clone(), abi_params(f).to_vec());
    }
    // The yield future's builder and poll function follow MIR's functions.
    let yield_poll = first + mir.functions.len() as u32 + 1;
    if yields {
        layout.yield_future = Some(yield_poll - 1);
    }
    if let Some(name) = &mir.runtime_alloc {
        layout.alloc = Some(
            *layout
                .funcs
                .get(name)
                .ok_or_else(|| anyhow!("the allocator `{name}` is not in the module"))?,
        );
    }
    for stmt in mir
        .functions
        .iter()
        .flat_map(|f| &f.blocks)
        .flat_map(|b| &b.stmts)
    {
        if let Stmt::CallIndirect { params, ret, .. } = stmt {
            let key = indirect_type(params, *ret);
            let ty = types.of(key.0.clone(), key.1.clone());
            layout.indirect.insert(key, ty);
        }
    }
    if polls || yields {
        let key = poll_type();
        let ty = types.of(key.0.clone(), key.1.clone());
        layout.indirect.insert(key, ty);
    }
    if calls {
        let key = closure_type();
        let ty = types.of(key.0.clone(), key.1.clone());
        layout.indirect.insert(key, ty);
    }
    // Every function whose address is taken, in first-use order: its code
    // pointer is its place in the table (spec §4.3).
    let mut table: Vec<u32> = Vec::new();
    for stmt in mir
        .functions
        .iter()
        .flat_map(|f| &f.blocks)
        .flat_map(|b| &b.stmts)
    {
        if let Stmt::MakeClosure { code, .. } = stmt {
            if !layout.table.contains_key(code) {
                let index = *layout
                    .funcs
                    .get(code)
                    .ok_or_else(|| anyhow!("a function value of an undeclared `{code}`"))?;
                layout.table.insert(code.clone(), table.len() as u32);
                table.push(index);
            }
        }
    }
    let yield_slot = table.len() as u32;
    if yields {
        table.push(yield_poll);
    }
    // A `call_indirect` needs a table, if an empty one.
    let indirect = !layout.indirect.is_empty();

    let mut functions = FunctionSection::new();
    let mut code = CodeSection::new();
    for f in &mir.functions {
        let ty = types.of(vals(abi_params(f)), val(f.ret).into_iter().collect());
        functions.function(ty);
        let body = FnLower::new(f, &layout).lower().map_err(|e| {
            if e.is::<Limit>() {
                e
            } else {
                anyhow!("compiling `{}`: {e}", f.name)
            }
        })?;
        code.function(&body);
    }
    if yields {
        let alloc = layout
            .alloc
            .ok_or_else(|| anyhow!("a yield future, but the module has no allocator"))?;
        functions.function(types.of(vec![], vec![ValType::I32]));
        code.function(&yield_future(alloc, yield_slot));
        let (params, results) = poll_type();
        functions.function(types.of(params, results));
        code.function(&yield_poll_body());
    }

    let mut module = wasm_encoder::Module::new();
    module.section(&types.section);
    module.section(&imports);
    module.section(&functions);
    if !table.is_empty() || indirect {
        let mut tables = TableSection::new();
        tables.table(TableType {
            element_type: RefType::FUNCREF,
            table64: false,
            minimum: table.len() as u64,
            maximum: Some(table.len() as u64),
            shared: false,
        });
        module.section(&tables);
    }
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: MEMORY_PAGES,
        maximum: Some(MEMORY_MAX_PAGES),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    module.section(&memories);
    let mut exports = ExportSection::new();
    let main = layout
        .funcs
        .get("main")
        .copied()
        .ok_or_else(|| anyhow!("the module has no `main`"))?;
    exports.export("main", ExportKind::Func, main);
    exports.export("memory", ExportKind::Memory, 0);
    if let Some(alloc) = layout.alloc {
        exports.export("nova_alloc", ExportKind::Func, alloc);
    }
    for (export, name) in &mir.runtime_exports {
        let index = *layout
            .funcs
            .get(name)
            .ok_or_else(|| anyhow!("the export `{export}` names `{name}`, not in the module"))?;
        exports.export(export, ExportKind::Func, index);
    }
    module.section(&exports);
    if !table.is_empty() {
        let mut elements = ElementSection::new();
        elements.active(
            Some(0),
            &ConstExpr::i32_const(0),
            Elements::Functions(Cow::Owned(table)),
        );
        module.section(&elements);
    }
    module.section(&code);
    if !data.is_empty() {
        let mut segments = DataSection::new();
        segments.active(0, &ConstExpr::i32_const(DATA_BASE as i32), data);
        module.section(&segments);
    }
    Ok(Output {
        wasm: module.finish(),
        imports: import_names,
    })
}

/// The yield future's builder, `() -> future` (spec 4.1 §4.3): a zeroed
/// two-slot state, and a future of the poll function at `slot` in the table
/// and that state.
fn yield_future(alloc: u32, slot: u32) -> Function {
    let mut f = Function::new([(2, ValType::I32)]);
    for (local, size) in [(0, 16), (1, 16)] {
        f.instruction(&I::I64Const(size));
        f.instruction(&I::Call(alloc));
        f.instruction(&I::I32WrapI64);
        f.instruction(&I::LocalSet(local));
    }
    f.instruction(&I::LocalGet(1));
    f.instruction(&I::I64Const(i64::from(slot)));
    f.instruction(&I::I64Store(mem(0, 3)));
    f.instruction(&I::LocalGet(1));
    f.instruction(&I::LocalGet(0));
    f.instruction(&I::I64ExtendI32U);
    f.instruction(&I::I64Store(mem(8, 3)));
    f.instruction(&I::LocalGet(1));
    f.instruction(&I::End);
    f
}

/// The yield future's poll function, `nova-runtime`'s `poll_yield_once`:
/// pending once, with no wait staged, then ready with the output 0.
fn yield_poll_body() -> Function {
    let mut f = Function::new([]);
    f.instruction(&I::LocalGet(0));
    f.instruction(&I::I64Load(mem(0, 3)));
    f.instruction(&I::I64Eqz);
    f.instruction(&I::If(BlockType::Result(ValType::I64)));
    f.instruction(&I::LocalGet(0));
    f.instruction(&I::I64Const(1));
    f.instruction(&I::I64Store(mem(0, 3)));
    f.instruction(&I::I64Const(0));
    f.instruction(&I::Else);
    f.instruction(&I::LocalGet(0));
    f.instruction(&I::I64Const(0));
    f.instruction(&I::I64Store(mem(8, 3)));
    f.instruction(&I::I64Const(1));
    f.instruction(&I::End);
    f.instruction(&I::End);
    f
}

/// A label the code being emitted is nested in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    /// The `loop` whose header is this block: `br` to it continues.
    Loop(usize),
    /// The `block` this block follows: `br` to it goes there.
    Block(usize),
    /// An `if`, which no branch targets.
    If,
}

/// One function's lowering.
struct FnLower<'a> {
    f: &'a MirFn,
    layout: &'a Layout,
    /// Each temp's local, or none for `Unit`.
    locals: Vec<Option<u32>>,
    /// The dispatch loop's block number, when the graph is irreducible.
    pc: u32,
    /// An `i32` local for an address a statement builds in two steps.
    scratch: u32,
    decls: Vec<(u32, ValType)>,
    out: Vec<I<'static>>,
    ctx: Vec<Ctx>,
    cfg: Cfg,
}

fn mem(offset: u64, align: u32) -> MemArg {
    MemArg {
        offset,
        align,
        memory_index: 0,
    }
}

impl<'a> FnLower<'a> {
    fn new(f: &'a MirFn, layout: &'a Layout) -> Self {
        let nparams = abi_params(f).len();
        let mut locals = vec![None; f.temps.len()];
        let mut next = 0u32;
        for (t, ty) in f.temps.iter().enumerate().take(nparams) {
            if val(*ty).is_some() {
                locals[t] = Some(next);
                next += 1;
            }
        }
        let mut decls = Vec::new();
        for (t, ty) in f.temps.iter().enumerate().skip(nparams) {
            if let Some(v) = val(*ty) {
                locals[t] = Some(next);
                next += 1;
                decls.push((1, v));
            }
        }
        let pc = next;
        let scratch = next + 1;
        decls.push((2, ValType::I32));
        FnLower {
            f,
            layout,
            locals,
            pc,
            scratch,
            decls,
            out: Vec::new(),
            ctx: Vec::new(),
            cfg: structure::analyze(&f.blocks),
        }
    }

    fn lower(mut self) -> Result<Function> {
        if self.f.blocks.is_empty() {
            bail!("a function with no blocks");
        }
        // Each value temp's local, and the two scratch `i32`s.
        let count = self.locals.iter().flatten().count() + 2;
        if count > MAX_LOCALS {
            return Err(Limit(format!(
                "function `{}` needs {count} locals, but WebAssembly allows {MAX_LOCALS}",
                self.f.name
            ))
            .into());
        }
        if self.cfg.reducible {
            self.tree(0)?;
        } else {
            self.dispatch()?;
        }
        // Every path above ends in `br`, `return` or `unreachable`, but
        // after an `end` WASM's validator sees the code as reachable again.
        self.out.push(I::Unreachable);
        let mut w = Function::new(self.decls.iter().copied());
        for ins in &self.out {
            w.instruction(ins);
        }
        w.instruction(&I::End);
        Ok(w)
    }

    // --- structuring (Ramsey, "Beyond Relooper") -------------------------

    /// The code for `x` and everything it dominates.
    fn tree(&mut self, x: usize) -> Result<()> {
        // Merge children, the latest in reverse postorder first: it is the
        // outermost `block`, so its code comes last.
        let mut merges: Vec<usize> = self.cfg.children[x]
            .iter()
            .copied()
            .filter(|&c| self.cfg.merge[c])
            .collect();
        merges.reverse();
        if self.cfg.loop_header[x] {
            self.out.push(I::Loop(BlockType::Empty));
            self.ctx.push(Ctx::Loop(x));
            self.within(x, &merges)?;
            self.ctx.pop();
            self.out.push(I::End);
            Ok(())
        } else {
            self.within(x, &merges)
        }
    }

    fn within(&mut self, x: usize, merges: &[usize]) -> Result<()> {
        match merges.split_first() {
            None => self.body(x),
            Some((&y, rest)) => {
                self.out.push(I::Block(BlockType::Empty));
                self.ctx.push(Ctx::Block(y));
                self.within(x, rest)?;
                self.ctx.pop();
                self.out.push(I::End);
                self.tree(y)
            }
        }
    }

    /// Go from `from` to `to`: continue a loop, leave to a merge node, or,
    /// for a block `from` alone reaches, its code in place.
    fn branch(&mut self, from: usize, to: usize) -> Result<()> {
        if self.cfg.rpo_index[to] <= self.cfg.rpo_index[from] {
            self.br(Ctx::Loop(to))
        } else if self.cfg.merge[to] {
            self.br(Ctx::Block(to))
        } else {
            self.tree(to)
        }
    }

    fn br(&mut self, target: Ctx) -> Result<()> {
        let at = self
            .ctx
            .iter()
            .rposition(|c| *c == target)
            .ok_or_else(|| anyhow!("no enclosing label for {target:?}"))?;
        self.out.push(I::Br((self.ctx.len() - 1 - at) as u32));
        Ok(())
    }

    /// Block `x`'s statements, then its terminator.
    fn body(&mut self, x: usize) -> Result<()> {
        let f = self.f;
        for s in &f.blocks[x].stmts {
            self.stmt(s)?;
        }
        match &f.blocks[x].term {
            Terminator::Goto(b) => self.branch(x, b.0 as usize),
            Terminator::Branch { cond, then_, else_ } => {
                self.get(*cond);
                self.out.push(I::If(BlockType::Empty));
                self.ctx.push(Ctx::If);
                self.branch(x, then_.0 as usize)?;
                self.out.push(I::Else);
                self.branch(x, else_.0 as usize)?;
                self.ctx.pop();
                self.out.push(I::End);
                Ok(())
            }
            Terminator::Switch {
                disc,
                arms,
                default,
            } => {
                for (v, b) in arms {
                    self.disc_eq(*disc, *v);
                    self.out.push(I::If(BlockType::Empty));
                    self.ctx.push(Ctx::If);
                    self.branch(x, b.0 as usize)?;
                    self.ctx.pop();
                    self.out.push(I::End);
                }
                self.branch(x, default.0 as usize)
            }
            Terminator::Return(t) => self.ret(*t),
            Terminator::Trap => {
                self.out.push(I::Unreachable);
                Ok(())
            }
        }
    }

    /// `disc == v`, as an `i32` condition.
    fn disc_eq(&mut self, disc: Temp, v: i64) {
        self.get(disc);
        if self.ty(disc) == MirTy::I64 {
            self.out.push(I::I64Const(v));
            self.out.push(I::I64Eq);
        } else {
            self.out.push(I::I32Const(v as i32));
            self.out.push(I::I32Eq);
        }
    }

    fn ret(&mut self, t: Option<Temp>) -> Result<()> {
        if val(self.f.ret).is_some() {
            match t {
                Some(t) if self.locals[t.0 as usize].is_some() => self.get(t),
                _ => bail!("a `return` without the value the function returns"),
            }
        }
        self.out.push(I::Return);
        Ok(())
    }

    // --- the dispatch loop, for an irreducible graph ----------------------

    /// Every block in one `loop`, chosen by `$pc` through a `br_table`.
    fn dispatch(&mut self) -> Result<()> {
        let f = self.f;
        let n = f.blocks.len() as u32;
        self.out.push(I::Loop(BlockType::Empty));
        for _ in 0..n {
            self.out.push(I::Block(BlockType::Empty));
        }
        self.out.push(I::LocalGet(self.pc));
        self.out
            .push(I::BrTable(Cow::Owned((0..n).collect()), n - 1));
        for (bi, block) in f.blocks.iter().enumerate() {
            self.out.push(I::End);
            // From block `bi`'s code, the loop is this many labels out.
            let to_loop = n - 1 - bi as u32;
            if self.cfg.rpo_index[bi] == UNREACHED {
                self.out.push(I::Unreachable);
                continue;
            }
            for s in &block.stmts {
                self.stmt(s)?;
            }
            let goto = |out: &mut Vec<I<'static>>, pc: u32, b: u32, depth: u32| {
                out.push(I::I32Const(b as i32));
                out.push(I::LocalSet(pc));
                out.push(I::Br(depth));
            };
            match &block.term {
                Terminator::Goto(b) => goto(&mut self.out, self.pc, b.0, to_loop),
                Terminator::Branch { cond, then_, else_ } => {
                    self.out.push(I::I32Const(then_.0 as i32));
                    self.out.push(I::I32Const(else_.0 as i32));
                    self.get(*cond);
                    self.out.push(I::Select);
                    self.out.push(I::LocalSet(self.pc));
                    self.out.push(I::Br(to_loop));
                }
                Terminator::Switch {
                    disc,
                    arms,
                    default,
                } => {
                    for (v, b) in arms {
                        self.disc_eq(*disc, *v);
                        self.out.push(I::If(BlockType::Empty));
                        goto(&mut self.out, self.pc, b.0, to_loop + 1);
                        self.out.push(I::End);
                    }
                    goto(&mut self.out, self.pc, default.0, to_loop);
                }
                Terminator::Return(t) => self.ret(*t)?,
                Terminator::Trap => self.out.push(I::Unreachable),
            }
        }
        self.out.push(I::End);
        Ok(())
    }

    // --- statements -------------------------------------------------------

    fn ty(&self, t: Temp) -> MirTy {
        self.f.temps[t.0 as usize]
    }

    fn get(&mut self, t: Temp) {
        if let Some(i) = self.locals[t.0 as usize] {
            self.out.push(I::LocalGet(i));
        }
    }

    fn set(&mut self, t: Temp) {
        if let Some(i) = self.locals[t.0 as usize] {
            self.out.push(I::LocalSet(i));
        }
    }

    /// Store a call's result in `dst`, or drop it.
    fn result(&mut self, dst: Option<Temp>, ret: MirTy) {
        if val(ret).is_none() {
            return;
        }
        match dst {
            Some(d) if self.locals[d.0 as usize].is_some() => self.set(d),
            _ => self.out.push(I::Drop),
        }
    }

    /// An `i64` address on the stack, as WASM's `i32`.
    fn wrap(&mut self, t: Temp) {
        self.get(t);
        self.out.push(I::I32WrapI64);
    }

    fn stmt(&mut self, s: &Stmt) -> Result<()> {
        match s {
            Stmt::ConstInt(t, v) => {
                self.out.push(I::I64Const(*v));
                self.set(*t);
            }
            Stmt::ConstFloat(t, v) => {
                self.out.push(I::F64Const((*v).into()));
                self.set(*t);
            }
            Stmt::ConstBool(t, v) => {
                self.out.push(I::I32Const(i32::from(*v)));
                self.set(*t);
            }
            Stmt::ConstUnit(_) => {}
            Stmt::Copy { dst, src } => {
                if self.locals[dst.0 as usize].is_some() {
                    self.get(*src);
                    self.set(*dst);
                }
            }
            Stmt::Bin {
                dst,
                op,
                class,
                lhs,
                rhs,
            } => {
                self.get(*lhs);
                self.get(*rhs);
                self.out.push(bin(*op, *class)?);
                self.set(*dst);
            }
            Stmt::Neg { dst, class, src } => {
                if *class == OperandClass::Float {
                    self.get(*src);
                    self.out.push(I::F64Neg);
                } else {
                    self.out.push(I::I64Const(0));
                    self.get(*src);
                    self.out.push(I::I64Sub);
                }
                self.set(*dst);
            }
            Stmt::Not { dst, src } => {
                self.get(*src);
                self.out.push(I::I32Eqz);
                self.set(*dst);
            }
            Stmt::BitNot { dst, src } => {
                self.get(*src);
                self.out.push(I::I64Const(-1));
                self.out.push(I::I64Xor);
                self.set(*dst);
            }
            Stmt::Call { dst, callee, args } => {
                let layout = self.layout;
                let index = *layout
                    .funcs
                    .get(callee)
                    .ok_or_else(|| anyhow!("a call to an undeclared `{callee}`"))?;
                // A call MIR routed to `std/rt` passes a `Ptr` where the
                // function takes an `Int` address, and may return the reverse
                // (spec §5.4).
                let params = &layout.params[callee];
                for (a, want) in args.iter().zip(params) {
                    self.get(*a);
                    self.convert(self.ty(*a), *want);
                }
                self.out.push(I::Call(index));
                let ret = layout.rets[callee];
                if let Some(d) = dst {
                    if val(ret).is_some() && self.locals[d.0 as usize].is_some() {
                        self.convert(ret, self.ty(*d));
                    }
                }
                self.result(*dst, ret);
            }
            Stmt::CallRuntime { dst, func, args } => {
                let index = match (func.on_wasm(), self.layout.yield_future) {
                    (WasmRuntime::Builtin, Some(index)) => index,
                    _ => *self
                        .layout
                        .hosts
                        .get(func)
                        .ok_or_else(|| anyhow!("`{}` is not a host function", func.symbol()))?,
                };
                for a in args {
                    self.get(*a);
                }
                self.out.push(I::Call(index));
                self.result(*dst, func.signature().1);
                if *func == RtFunc::Panic {
                    self.out.push(I::Unreachable);
                }
            }
            Stmt::Mem { dst, op, args } => self.mem(*dst, *op, args)?,
            Stmt::ConstStr(t, text) => {
                let at = self.layout.literals[text];
                self.out.push(I::I32Const(at as i32));
                self.set(*t);
            }
            Stmt::MakeSum { dst, tag, fields } => {
                self.alloc(8 + 8 * fields.len() as i64)?;
                self.set(*dst);
                self.get(*dst);
                self.out.push(I::I64Const(i64::from(*tag)));
                self.out.push(I::I64Store(mem(0, 3)));
                for (i, (v, ty)) in fields.iter().enumerate() {
                    self.store_field(*dst, 8 + 8 * i as u64, *v, *ty);
                }
            }
            Stmt::MakeRecord { dst, fields } => {
                self.alloc(8 * fields.len() as i64)?;
                self.set(*dst);
                for (i, (v, ty)) in fields.iter().enumerate() {
                    self.store_field(*dst, 8 * i as u64, *v, *ty);
                }
            }
            Stmt::MakeArray { dst, elems } => {
                self.alloc(8 + 8 * elems.len() as i64)?;
                self.set(*dst);
                self.get(*dst);
                self.out.push(I::I64Const(elems.len() as i64));
                self.out.push(I::I64Store(mem(0, 3)));
                for (i, (v, ty)) in elems.iter().enumerate() {
                    self.store_field(*dst, 8 + 8 * i as u64, *v, *ty);
                }
            }
            Stmt::ArrayAlloc { dst, len } => {
                // `8 + 8*len` bytes, `len` at offset 0. MIR has checked
                // `len`'s range, and the allocator zeroes.
                let alloc = self.alloc_index()?;
                self.get(*len);
                self.out.push(I::I64Const(8));
                self.out.push(I::I64Mul);
                self.out.push(I::I64Const(8));
                self.out.push(I::I64Add);
                self.out.push(I::Call(alloc));
                self.out.push(I::I32WrapI64);
                self.set(*dst);
                self.get(*dst);
                self.get(*len);
                self.out.push(I::I64Store(mem(0, 3)));
            }
            Stmt::MakeClosure {
                dst,
                code,
                captures,
            } => {
                // The environment: the captures in 8-byte slots, or null.
                if captures.is_empty() {
                    self.out.push(I::I32Const(0));
                    self.out.push(I::LocalSet(self.scratch));
                } else {
                    self.alloc(8 * captures.len() as i64)?;
                    self.out.push(I::LocalSet(self.scratch));
                    for (i, (v, ty)) in captures.iter().enumerate() {
                        self.store_at(self.scratch, 8 * i as u64, *v, *ty);
                    }
                }
                // `{ code, env }`: the code is the function's table index.
                let slot = self.layout.table[code];
                self.alloc(16)?;
                self.set(*dst);
                self.get(*dst);
                self.out.push(I::I64Const(i64::from(slot)));
                self.out.push(I::I64Store(mem(0, 3)));
                self.get(*dst);
                self.out.push(I::LocalGet(self.scratch));
                self.out.push(I::I64ExtendI32U);
                self.out.push(I::I64Store(mem(8, 3)));
            }
            Stmt::CallIndirect {
                dst,
                callee,
                params,
                ret,
                args,
            } => {
                self.get(*callee);
                self.out.push(I::I32Load(mem(8, 2)));
                for a in args {
                    self.get(*a);
                }
                self.get(*callee);
                self.out.push(I::I32Load(mem(0, 2)));
                let ty = self.layout.indirect[&indirect_type(params, *ret)];
                self.out.push(I::CallIndirect {
                    type_index: ty,
                    table_index: 0,
                });
                self.result(*dst, *ret);
            }
            Stmt::SumTag { dst, sum } => {
                self.get(*sum);
                self.out.push(I::I64Load(mem(0, 3)));
                self.set(*dst);
            }
            Stmt::SumField {
                dst,
                sum,
                index,
                ty,
            } => self.load_field(*dst, *sum, 8 + 8 * u64::from(*index), *ty),
            Stmt::RecordField {
                dst,
                record,
                index,
                ty,
            } => self.load_field(*dst, *record, 8 * u64::from(*index), *ty),
            Stmt::SetField {
                record,
                index,
                value,
                ty,
            } => self.store_field(*record, 8 * u64::from(*index), *value, *ty),
            Stmt::ArrayLen { dst, arr } => {
                self.get(*arr);
                self.out.push(I::I64Load(mem(0, 3)));
                self.set(*dst);
            }
            Stmt::ArrayGet {
                dst,
                arr,
                index,
                ty,
            } => {
                if val(*ty).is_some() {
                    self.elem_addr(*arr, *index);
                    self.out.push(I::LocalSet(self.scratch));
                    self.load_at(*dst, self.scratch, 8, *ty);
                }
            }
            Stmt::ArraySet {
                arr,
                index,
                value,
                ty,
            } => {
                if val(*ty).is_some() {
                    self.elem_addr(*arr, *index);
                    self.out.push(I::LocalSet(self.scratch));
                    self.store_at(self.scratch, 8, *value, *ty);
                }
            }
            Stmt::Await { .. } => bail!(
                "an `.await` marker reached codegen: `nova-mir`'s async state-machine \
                 transform must split every `async fn` body at its await points first"
            ),
        }
        Ok(())
    }

    /// Convert the value on the stack from `from`'s class to `to`'s, where a
    /// `Ptr` meets an `Int` address (spec §5.4); otherwise nothing.
    fn convert(&mut self, from: MirTy, to: MirTy) {
        match (from, to) {
            (MirTy::Ptr, MirTy::I64) => self.out.push(I::I64ExtendI32U),
            (MirTy::I64, MirTy::Ptr) => self.out.push(I::I32WrapI64),
            _ => {}
        }
    }

    fn alloc_index(&self) -> Result<u32> {
        self.layout
            .alloc
            .ok_or_else(|| anyhow!("a heap object, but the module has no allocator"))
    }

    /// Allocate `size` bytes; the object's address is left on the stack.
    fn alloc(&mut self, size: i64) -> Result<()> {
        let alloc = self.alloc_index()?;
        self.out.push(I::I64Const(size));
        self.out.push(I::Call(alloc));
        self.out.push(I::I32WrapI64);
        Ok(())
    }

    /// `arr + 8*index`, the address of element `index` less its 8-byte
    /// header, as an `i32`.
    fn elem_addr(&mut self, arr: Temp, index: Temp) {
        self.get(arr);
        self.get(index);
        self.out.push(I::I32WrapI64);
        self.out.push(I::I32Const(3));
        self.out.push(I::I32Shl);
        self.out.push(I::I32Add);
    }

    /// Store `v` of class `ty` at `object + offset`.
    fn store_field(&mut self, object: Temp, offset: u64, v: Temp, ty: MirTy) {
        if let Some(i) = self.locals[object.0 as usize] {
            self.store_at(i, offset, v, ty);
        }
    }

    /// Store `v` of class `ty` at `local + offset`, `local` an `i32`
    /// address. A `Ptr` is stored zero-extended to 8 bytes (spec §4.1).
    fn store_at(&mut self, local: u32, offset: u64, v: Temp, ty: MirTy) {
        if val(ty).is_none() {
            return;
        }
        self.out.push(I::LocalGet(local));
        self.get(v);
        match ty {
            MirTy::I64 => self.out.push(I::I64Store(mem(offset, 3))),
            MirTy::F64 => self.out.push(I::F64Store(mem(offset, 3))),
            MirTy::I8 => self.out.push(I::I32Store8(mem(offset, 0))),
            MirTy::Ptr => {
                self.out.push(I::I64ExtendI32U);
                self.out.push(I::I64Store(mem(offset, 3)));
            }
            MirTy::Unit => unreachable!("handled above"),
        }
    }

    /// Load `object + offset` of class `ty` into `dst`.
    fn load_field(&mut self, dst: Temp, object: Temp, offset: u64, ty: MirTy) {
        if let Some(i) = self.locals[object.0 as usize] {
            self.load_at(dst, i, offset, ty);
        }
    }

    /// Load `local + offset` of class `ty` into `dst`.
    fn load_at(&mut self, dst: Temp, local: u32, offset: u64, ty: MirTy) {
        if val(ty).is_none() || self.locals[dst.0 as usize].is_none() {
            return;
        }
        self.out.push(I::LocalGet(local));
        self.out.push(match ty {
            MirTy::I64 => I::I64Load(mem(offset, 3)),
            MirTy::F64 => I::F64Load(mem(offset, 3)),
            MirTy::I8 => I::I32Load8U(mem(offset, 0)),
            MirTy::Ptr => I::I32Load(mem(offset, 2)),
            MirTy::Unit => unreachable!("handled above"),
        });
        self.set(dst);
    }

    /// A memory access in line (spec §5.1).
    fn mem(&mut self, dst: Option<Temp>, op: MemOp, args: &[Temp]) -> Result<()> {
        let arg = |i: usize| -> Result<Temp> {
            args.get(i)
                .copied()
                .ok_or_else(|| anyhow!("{op:?} takes more arguments"))
        };
        match op {
            MemOp::Load8 => {
                self.wrap(arg(0)?);
                self.out.push(I::I64Load8U(mem(0, 0)));
            }
            MemOp::Load64 => {
                self.wrap(arg(0)?);
                self.out.push(I::I64Load(mem(0, 3)));
            }
            MemOp::Store8 => {
                self.wrap(arg(0)?);
                self.get(arg(1)?);
                self.out.push(I::I64Store8(mem(0, 0)));
            }
            MemOp::Store64 => {
                self.wrap(arg(0)?);
                self.get(arg(1)?);
                self.out.push(I::I64Store(mem(0, 3)));
            }
            MemOp::Copy => {
                self.wrap(arg(0)?);
                self.wrap(arg(1)?);
                self.wrap(arg(2)?);
                self.out.push(I::MemoryCopy {
                    src_mem: 0,
                    dst_mem: 0,
                });
            }
            MemOp::Fill => {
                self.wrap(arg(0)?);
                self.wrap(arg(1)?);
                self.wrap(arg(2)?);
                self.out.push(I::MemoryFill(0));
            }
            MemOp::Pages => {
                self.out.push(I::MemorySize(0));
                self.out.push(I::I64ExtendI32U);
            }
            MemOp::Grow => {
                self.wrap(arg(0)?);
                self.out.push(I::MemoryGrow(0));
                // -1, "cannot grow", stays -1.
                self.out.push(I::I64ExtendI32S);
            }
            MemOp::HeapBase => self.out.push(I::I64Const(self.layout.heap_base)),
            MemOp::Poll => {
                // `poll(state, null)`, the future's table index its code.
                self.wrap(arg(0)?);
                self.out.push(I::LocalSet(self.scratch));
                self.out.push(I::LocalGet(self.scratch));
                self.out.push(I::I32Load(mem(8, 2)));
                self.out.push(I::I32Const(0));
                self.out.push(I::LocalGet(self.scratch));
                self.out.push(I::I32Load(mem(0, 2)));
                self.out.push(I::CallIndirect {
                    type_index: self.layout.indirect[&poll_type()],
                    table_index: 0,
                });
            }
            MemOp::Abort => {
                let abort = self
                    .layout
                    .abort
                    .ok_or_else(|| anyhow!("an abort, but the module does not import one"))?;
                self.out.push(I::Call(abort));
                self.out.push(I::Unreachable);
            }
            MemOp::ClosureWord => {
                self.get(arg(0)?);
                self.out.push(I::I64ExtendI32U);
            }
            MemOp::ClosureCall => {
                // `code(env, arg)`, as `CallIndirect` calls a closure.
                self.wrap(arg(0)?);
                self.out.push(I::LocalSet(self.scratch));
                self.out.push(I::LocalGet(self.scratch));
                self.out.push(I::I32Load(mem(8, 2)));
                self.get(arg(1)?);
                self.out.push(I::LocalGet(self.scratch));
                self.out.push(I::I32Load(mem(0, 2)));
                self.out.push(I::CallIndirect {
                    type_index: self.layout.indirect[&closure_type()],
                    table_index: 0,
                });
            }
        }
        let produces = matches!(
            op,
            MemOp::Load8
                | MemOp::Load64
                | MemOp::Pages
                | MemOp::Grow
                | MemOp::HeapBase
                | MemOp::Poll
                | MemOp::ClosureWord
        );
        if produces {
            self.result(dst, MirTy::I64);
        }
        Ok(())
    }
}

/// The instruction for `lhs op rhs`.
fn bin(op: BinOp, class: OperandClass) -> Result<I<'static>> {
    use BinOp::*;
    Ok(match (class, op) {
        (OperandClass::Float, Add) => I::F64Add,
        (OperandClass::Float, Sub) => I::F64Sub,
        (OperandClass::Float, Mul) => I::F64Mul,
        (OperandClass::Float, Div) => I::F64Div,
        (OperandClass::Float, Rem) => {
            bail!("float remainder (%) is not supported yet by the WASM backend")
        }
        (OperandClass::Float, Eq) => I::F64Eq,
        (OperandClass::Float, Ne) => I::F64Ne,
        (OperandClass::Float, Lt) => I::F64Lt,
        (OperandClass::Float, Le) => I::F64Le,
        (OperandClass::Float, Gt) => I::F64Gt,
        (OperandClass::Float, Ge) => I::F64Ge,
        (OperandClass::Float, _) => bail!("bitwise operators are not defined for Float"),
        (OperandClass::Bool, Eq) => I::I32Eq,
        (OperandClass::Bool, Ne) => I::I32Ne,
        (OperandClass::Bool, BitAnd) => I::I32And,
        (OperandClass::Bool, BitOr) => I::I32Or,
        (OperandClass::Bool, BitXor) => I::I32Xor,
        (OperandClass::Bool, o) => bail!("operator {o:?} is not defined for Bool"),
        (OperandClass::Int, Add) => I::I64Add,
        (OperandClass::Int, Sub) => I::I64Sub,
        (OperandClass::Int, Mul) => I::I64Mul,
        (OperandClass::Int, Div) => I::I64DivS,
        (OperandClass::Int, Rem) => I::I64RemS,
        (OperandClass::Int, Eq) => I::I64Eq,
        (OperandClass::Int, Ne) => I::I64Ne,
        (OperandClass::Int, Lt) => I::I64LtS,
        (OperandClass::Int, Le) => I::I64LeS,
        (OperandClass::Int, Gt) => I::I64GtS,
        (OperandClass::Int, Ge) => I::I64GeS,
        (OperandClass::Int, BitAnd) => I::I64And,
        (OperandClass::Int, BitOr) => I::I64Or,
        (OperandClass::Int, BitXor) => I::I64Xor,
        (OperandClass::Int, Shl) => I::I64Shl,
        (OperandClass::Int, Shr) => I::I64ShrS,
    })
}

/// `node`, for the tests that run what they build, or `None` when it is
/// not on `PATH`, in which case the test passes without running; under
/// `NOVA_REQUIRE_NODE=1`, as in CI, a missing `node` fails it instead (spec
/// §9.1).
#[cfg(test)]
pub(crate) fn test_node() -> Option<std::process::Command> {
    let found = std::process::Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if found {
        return Some(std::process::Command::new("node"));
    }
    assert_ne!(
        std::env::var("NOVA_REQUIRE_NODE").as_deref(),
        Ok("1"),
        "NOVA_REQUIRE_NODE=1, but `node` is not on PATH"
    );
    eprintln!("skipped: `node` is not on PATH");
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_mir::{Block, BlockId};

    fn func(name: &str, params: u32, temps: Vec<MirTy>, ret: MirTy, blocks: Vec<Block>) -> MirFn {
        MirFn {
            name: name.to_string(),
            params,
            takes_env: false,
            capture_count: 0,
            temps,
            ret,
            is_async: false,
            blocks,
        }
    }

    fn block(stmts: Vec<Stmt>, term: Terminator) -> Block {
        Block { stmts, term }
    }

    fn module(functions: Vec<MirFn>) -> Module {
        Module {
            functions,
            ..Default::default()
        }
    }

    /// Compile and validate, returning the module's bytes. The validator
    /// takes the MVP and bulk memory alone, the one feature `nova build
    /// --release` enables for `wasm-opt` (`WASM_OPT_FLAGS` in
    /// `crates/nova-cli/src/bundle.rs`; the 4.8 spec
    /// `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
    /// §4): a module that needs another fails here, and its flag goes there.
    fn valid(m: &Module) -> Output {
        let out = compile_module(m).expect("compiles");
        let features = wasmparser::WasmFeatures::MVP | wasmparser::WasmFeatures::BULK_MEMORY;
        wasmparser::Validator::new_with_features(features)
            .validate_all(&out.wasm)
            .expect("a valid module");
        out
    }

    fn ret_unit() -> Terminator {
        Terminator::Return(None)
    }

    /// Every scalar operator of each class.
    #[test]
    fn scalar_operators_lower_to_a_valid_module() {
        use BinOp::*;
        let mut stmts = vec![
            Stmt::ConstInt(Temp(0), 7),
            Stmt::ConstInt(Temp(1), 3),
            Stmt::ConstFloat(Temp(2), 1.5),
            Stmt::ConstBool(Temp(5), true),
        ];
        for op in [Add, Sub, Mul, Div, Rem, BitAnd, BitOr, BitXor, Shl, Shr] {
            stmts.push(Stmt::Bin {
                dst: Temp(0),
                op,
                class: OperandClass::Int,
                lhs: Temp(0),
                rhs: Temp(1),
            });
        }
        for op in [Eq, Ne, Lt, Le, Gt, Ge] {
            stmts.push(Stmt::Bin {
                dst: Temp(5),
                op,
                class: OperandClass::Int,
                lhs: Temp(0),
                rhs: Temp(1),
            });
            stmts.push(Stmt::Bin {
                dst: Temp(5),
                op,
                class: OperandClass::Float,
                lhs: Temp(2),
                rhs: Temp(2),
            });
        }
        for op in [Add, Sub, Mul, Div] {
            stmts.push(Stmt::Bin {
                dst: Temp(2),
                op,
                class: OperandClass::Float,
                lhs: Temp(2),
                rhs: Temp(2),
            });
        }
        stmts.extend([
            Stmt::Neg {
                dst: Temp(0),
                class: OperandClass::Int,
                src: Temp(0),
            },
            Stmt::Neg {
                dst: Temp(2),
                class: OperandClass::Float,
                src: Temp(2),
            },
            Stmt::BitNot {
                dst: Temp(0),
                src: Temp(0),
            },
            Stmt::Not {
                dst: Temp(5),
                src: Temp(5),
            },
            Stmt::Bin {
                dst: Temp(5),
                op: Eq,
                class: OperandClass::Bool,
                lhs: Temp(5),
                rhs: Temp(5),
            },
            Stmt::ConstUnit(Temp(3)),
            Stmt::Copy {
                dst: Temp(4),
                src: Temp(0),
            },
        ]);
        let temps = vec![
            MirTy::I64,
            MirTy::I64,
            MirTy::F64,
            MirTy::Unit,
            MirTy::I64,
            MirTy::I8,
        ];
        valid(&module(vec![func(
            "main",
            0,
            temps,
            MirTy::Unit,
            vec![block(stmts, ret_unit())],
        )]));
    }

    /// A call passes no `Unit` argument, and a result nobody keeps is
    /// dropped.
    #[test]
    fn a_call_passes_no_unit_and_drops_an_unkept_result() {
        let callee = func(
            "f.1",
            2,
            vec![MirTy::Unit, MirTy::I64],
            MirTy::I64,
            vec![block(vec![], Terminator::Return(Some(Temp(1))))],
        );
        let main = func(
            "main",
            0,
            vec![MirTy::Unit, MirTy::I64],
            MirTy::Unit,
            vec![block(
                vec![
                    Stmt::ConstUnit(Temp(0)),
                    Stmt::ConstInt(Temp(1), 4),
                    Stmt::Call {
                        dst: None,
                        callee: "f.1".to_string(),
                        args: vec![Temp(0), Temp(1)],
                    },
                    Stmt::Call {
                        dst: Some(Temp(1)),
                        callee: "f.1".to_string(),
                        args: vec![Temp(0), Temp(1)],
                    },
                ],
                ret_unit(),
            )],
        );
        valid(&module(vec![main, callee]));
    }

    /// `while i < 10 { if i == 5 { break } ; i = i + 1 }`, with a
    /// `continue` path and a nested loop: the shapes MIR builds.
    #[test]
    fn loops_with_break_and_continue_are_structured() {
        // 0: i = 0 → 1
        // 1: (outer header) c = i < 10 → 2 | 6
        // 2: c = i == 5 → 6 (break) | 3
        // 3: (inner header) c = i < 3 → 4 | 5
        // 4: i = i + 1 → 3 (continue inner)
        // 5: i = i + 1 → 1 (continue outer)
        // 6: return
        let lt = |n| Stmt::ConstInt(Temp(2), n);
        let cmp = |op| Stmt::Bin {
            dst: Temp(1),
            op,
            class: OperandClass::Int,
            lhs: Temp(0),
            rhs: Temp(2),
        };
        let inc = Stmt::Bin {
            dst: Temp(0),
            op: BinOp::Add,
            class: OperandClass::Int,
            lhs: Temp(0),
            rhs: Temp(3),
        };
        let br = |t, e| Terminator::Branch {
            cond: Temp(1),
            then_: BlockId(t),
            else_: BlockId(e),
        };
        let blocks = vec![
            block(
                vec![Stmt::ConstInt(Temp(0), 0), Stmt::ConstInt(Temp(3), 1)],
                Terminator::Goto(BlockId(1)),
            ),
            block(vec![lt(10), cmp(BinOp::Lt)], br(2, 6)),
            block(vec![lt(5), cmp(BinOp::Eq)], br(6, 3)),
            block(vec![lt(3), cmp(BinOp::Lt)], br(4, 5)),
            block(vec![inc.clone_stmt()], Terminator::Goto(BlockId(3))),
            block(vec![inc.clone_stmt()], Terminator::Goto(BlockId(1))),
            block(vec![], ret_unit()),
        ];
        let f = func(
            "main",
            0,
            vec![MirTy::I64, MirTy::I8, MirTy::I64, MirTy::I64],
            MirTy::Unit,
            blocks,
        );
        let cfg = structure::analyze(&f.blocks);
        assert!(cfg.reducible);
        valid(&module(vec![f]));
    }

    /// A dense `Switch` and a sparse one, each with a default.
    #[test]
    fn switches_are_structured() {
        for arms in [vec![0, 1, 2], vec![-7, 40, 1000]] {
            let mut blocks = vec![block(
                vec![Stmt::ConstInt(Temp(0), 1)],
                Terminator::Switch {
                    disc: Temp(0),
                    arms: arms
                        .iter()
                        .enumerate()
                        .map(|(i, v)| (*v, BlockId(i as u32 + 1)))
                        .collect(),
                    default: BlockId(arms.len() as u32 + 1),
                },
            )];
            for _ in 0..=arms.len() {
                blocks.push(block(
                    vec![],
                    Terminator::Goto(BlockId(arms.len() as u32 + 2)),
                ));
            }
            blocks.push(block(vec![], ret_unit()));
            valid(&module(vec![func(
                "main",
                0,
                vec![MirTy::I64],
                MirTy::Unit,
                blocks,
            )]));
        }
    }

    /// Two entries into one cycle: the function falls back to a dispatch
    /// loop, which still validates.
    #[test]
    fn an_irreducible_graph_falls_back_to_a_dispatch_loop() {
        let blocks = vec![
            block(
                vec![Stmt::ConstBool(Temp(0), true)],
                Terminator::Branch {
                    cond: Temp(0),
                    then_: BlockId(1),
                    else_: BlockId(2),
                },
            ),
            block(
                vec![],
                Terminator::Branch {
                    cond: Temp(0),
                    then_: BlockId(2),
                    else_: BlockId(3),
                },
            ),
            block(vec![], Terminator::Goto(BlockId(1))),
            block(vec![], ret_unit()),
        ];
        let f = func("main", 0, vec![MirTy::I8], MirTy::Unit, blocks);
        assert!(!structure::analyze(&f.blocks).reducible);
        let out = valid(&module(vec![f]));
        let has_br_table = wasmparser::Parser::new(0)
            .parse_all(&out.wasm)
            .filter_map(Result::ok)
            .any(|p| match p {
                wasmparser::Payload::CodeSectionEntry(body) => body
                    .get_operators_reader()
                    .map(|ops| {
                        ops.into_iter()
                            .filter_map(Result::ok)
                            .any(|op| matches!(op, wasmparser::Operator::BrTable { .. }))
                    })
                    .unwrap_or(false),
                _ => false,
            });
        assert!(has_br_table, "the fallback dispatches through a br_table");
    }

    /// The dispatch loop computes, not only validates: an irreducible graph
    /// that counts to five runs under Node and stores 5 (spec §4.2).
    #[test]
    fn an_irreducible_graph_runs_under_node() {
        let Some(mut node) = crate::test_node() else {
            return;
        };
        let add = |dst, op, lhs, rhs| Stmt::Bin {
            dst: Temp(dst),
            op,
            class: OperandClass::Int,
            lhs: Temp(lhs),
            rhs: Temp(rhs),
        };
        let blocks = vec![
            block(
                vec![
                    Stmt::ConstInt(Temp(0), 0),
                    Stmt::ConstInt(Temp(1), 1),
                    Stmt::ConstInt(Temp(2), 5),
                    Stmt::ConstInt(Temp(3), i64::from(DATA_BASE)),
                    Stmt::ConstBool(Temp(4), true),
                ],
                Terminator::Branch {
                    cond: Temp(4),
                    then_: BlockId(1),
                    else_: BlockId(2),
                },
            ),
            block(vec![add(0, BinOp::Add, 0, 1)], Terminator::Goto(BlockId(2))),
            block(
                vec![add(5, BinOp::Lt, 0, 2)],
                Terminator::Branch {
                    cond: Temp(5),
                    then_: BlockId(1),
                    else_: BlockId(3),
                },
            ),
            block(
                vec![Stmt::Mem {
                    dst: None,
                    op: MemOp::Store64,
                    args: vec![Temp(3), Temp(0)],
                }],
                ret_unit(),
            ),
        ];
        let temps = vec![
            MirTy::I64,
            MirTy::I64,
            MirTy::I64,
            MirTy::I64,
            MirTy::I8,
            MirTy::I8,
        ];
        let f = func("main", 0, temps, MirTy::Unit, blocks);
        assert!(!structure::analyze(&f.blocks).reducible);
        let out = valid(&module(vec![f]));
        let dir = std::env::temp_dir().join("nova-codegen-wasm-dispatch");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        std::fs::write(dir.join("app.wasm"), &out.wasm).expect("the module");
        let script = format!(
            "const fs = require(\"fs\");\n\
             WebAssembly.instantiate(fs.readFileSync(process.argv[2]), {{ env: {{}} }}).then(({{ instance }}) => {{\n  \
             instance.exports.main();\n  \
             console.log(new DataView(instance.exports.memory.buffer).getBigInt64({DATA_BASE}, true).toString());\n\
             }});\n"
        );
        std::fs::write(dir.join("run.cjs"), script).expect("the script");
        let out = node
            .arg(dir.join("run.cjs"))
            .arg(dir.join("app.wasm"))
            .output()
            .expect("node runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&out.stdout), "5\n");
    }

    /// More locals than WebAssembly allows is a [`Limit`], naming the
    /// function; at the limit, the module is valid.
    #[test]
    fn a_function_over_the_local_limit_is_refused() {
        let over = func(
            "main",
            0,
            vec![MirTy::I64; MAX_LOCALS - 1],
            MirTy::Unit,
            vec![block(vec![], ret_unit())],
        );
        let e = compile_module(&module(vec![over])).expect_err("refused");
        assert!(e.is::<Limit>(), "{e:#}");
        assert_eq!(
            e.to_string(),
            "function `main` needs 50001 locals, but WebAssembly allows 50000"
        );
        let at = func(
            "main",
            0,
            vec![MirTy::I64; MAX_LOCALS - 2],
            MirTy::Unit,
            vec![block(vec![], ret_unit())],
        );
        valid(&module(vec![at]));
    }

    /// A long run of joins: the structuring recurses once per join, which
    /// must not overflow the stack of the thread that compiles.
    #[test]
    fn a_long_chain_of_joins_compiles() {
        const JOINS: u32 = 20_000;
        let mut blocks = Vec::new();
        for i in 0..JOINS {
            let b = 3 * i;
            blocks.push(block(
                vec![],
                Terminator::Branch {
                    cond: Temp(0),
                    then_: BlockId(b + 1),
                    else_: BlockId(b + 2),
                },
            ));
            blocks.push(block(vec![], Terminator::Goto(BlockId(b + 3))));
            blocks.push(block(vec![], Terminator::Goto(BlockId(b + 3))));
        }
        blocks.push(block(vec![], ret_unit()));
        blocks[0].stmts.push(Stmt::ConstBool(Temp(0), true));
        valid(&module(vec![func(
            "main",
            0,
            vec![MirTy::I8],
            MirTy::Unit,
            blocks,
        )]));
    }

    /// Every memory builtin, in line.
    #[test]
    fn memory_accesses_lower_in_line() {
        let a = |op, args: Vec<u32>, dst: Option<u32>| Stmt::Mem {
            dst: dst.map(Temp),
            op,
            args: args.into_iter().map(Temp).collect(),
        };
        let stmts = vec![
            Stmt::ConstInt(Temp(0), 64),
            Stmt::ConstInt(Temp(1), 9),
            a(MemOp::Store64, vec![0, 1], None),
            a(MemOp::Load64, vec![0], Some(1)),
            a(MemOp::Store8, vec![0, 1], None),
            a(MemOp::Load8, vec![0], Some(1)),
            a(MemOp::Copy, vec![0, 0, 1], None),
            a(MemOp::Fill, vec![0, 1, 1], None),
            a(MemOp::Pages, vec![], Some(1)),
            a(MemOp::Grow, vec![1], Some(1)),
            a(MemOp::HeapBase, vec![], Some(1)),
        ];
        valid(&module(vec![func(
            "main",
            0,
            vec![MirTy::I64, MirTy::I64],
            MirTy::Unit,
            vec![block(stmts, ret_unit())],
        )]));
    }

    /// A host function is imported from `env` once, under its field name.
    #[test]
    fn a_host_function_is_imported_from_env() {
        let call = |func| Stmt::CallRuntime {
            dst: None,
            func,
            args: vec![Temp(0)],
        };
        let f = func(
            "main",
            0,
            vec![MirTy::Ptr],
            MirTy::Unit,
            vec![block(
                vec![
                    call(RtFunc::Println),
                    call(RtFunc::Print),
                    call(RtFunc::Println),
                ],
                ret_unit(),
            )],
        );
        let out = valid(&module(vec![f]));
        assert_eq!(out.imports, ["println", "print"]);
    }

    /// std's `extern "js"` functions are imported after the host functions
    /// and before `abort`, under their symbols, and MIR calls them by name
    /// (spec 4.2 §3.1).
    #[test]
    fn an_extern_js_function_is_imported_after_the_host_functions() {
        let f = func(
            "main",
            0,
            vec![MirTy::Ptr, MirTy::I64],
            MirTy::Unit,
            vec![block(
                vec![
                    Stmt::CallRuntime {
                        dst: None,
                        func: RtFunc::Println,
                        args: vec![Temp(0)],
                    },
                    Stmt::Call {
                        dst: Some(Temp(1)),
                        callee: "dom_create_text".to_string(),
                        args: vec![Temp(0)],
                    },
                    Stmt::Mem {
                        dst: None,
                        op: MemOp::Abort,
                        args: vec![],
                    },
                ],
                ret_unit(),
            )],
        );
        let mut m = module(vec![f]);
        m.externs.push(nova_mir::ExternDecl {
            symbol: "dom_create_text".to_string(),
            abi: ExternAbi::Js,
            params: vec![MirTy::Ptr],
            ret: MirTy::I64,
        });
        let out = valid(&m);
        assert_eq!(out.imports, ["println", "dom_create_text", "abort"]);
        m.externs[0].abi = ExternAbi::C;
        let e = compile_module(&m).expect_err("refused");
        assert!(
            e.to_string()
                .contains("extern \"C\" function `dom_create_text`"),
            "{e}"
        );
    }

    /// `host_abort` imports the shim's `abort` after the host functions,
    /// once (spec 4.1 §4.5).
    #[test]
    fn an_abort_imports_the_shims_abort() {
        let abort = || Stmt::Mem {
            dst: None,
            op: MemOp::Abort,
            args: vec![],
        };
        let print = Stmt::CallRuntime {
            dst: None,
            func: RtFunc::EPrint,
            args: vec![Temp(0)],
        };
        let f = func(
            "main",
            0,
            vec![MirTy::Ptr],
            MirTy::Unit,
            vec![block(vec![print, abort(), abort()], ret_unit())],
        );
        let out = valid(&module(vec![f]));
        assert_eq!(out.imports, ["eprint", "abort"]);
    }

    /// `future_poll` with no function value in the module still has a
    /// table, empty, to call through (spec 4.1 §4.3).
    #[test]
    fn a_poll_without_a_function_value_validates() {
        let f = func(
            "main",
            0,
            vec![MirTy::I64, MirTy::I64],
            MirTy::Unit,
            vec![block(
                vec![
                    Stmt::ConstInt(Temp(0), 0),
                    Stmt::Mem {
                        dst: Some(Temp(1)),
                        op: MemOp::Poll,
                        args: vec![Temp(0)],
                    },
                ],
                ret_unit(),
            )],
        );
        valid(&module(vec![f]));
    }

    /// The yield future the backend builds is pending once, then ready, as
    /// `future_poll` reports it, run under Node (spec 4.1 §4.3, §9.3).
    #[test]
    fn the_yield_future_is_pending_once_then_ready() {
        // `addr(x) = x`: a call converts the future's `Ptr` to the `Int`
        // address a poll takes.
        let addr = func(
            "addr",
            1,
            vec![MirTy::I64],
            MirTy::I64,
            vec![block(vec![], Terminator::Return(Some(Temp(0))))],
        );
        // t0 future, t1 its address, t2 and t3 the two statuses, t4 and t5
        // where they go.
        let poll = |dst| Stmt::Mem {
            dst: Some(Temp(dst)),
            op: MemOp::Poll,
            args: vec![Temp(1)],
        };
        let store = |at, v| Stmt::Mem {
            dst: None,
            op: MemOp::Store64,
            args: vec![Temp(at), Temp(v)],
        };
        let main = func(
            "main",
            0,
            vec![
                MirTy::Ptr,
                MirTy::I64,
                MirTy::I64,
                MirTy::I64,
                MirTy::I64,
                MirTy::I64,
            ],
            MirTy::Unit,
            vec![block(
                vec![
                    Stmt::CallRuntime {
                        dst: Some(Temp(0)),
                        func: RtFunc::TaskYieldFuture,
                        args: vec![],
                    },
                    Stmt::Call {
                        dst: Some(Temp(1)),
                        callee: "addr".to_string(),
                        args: vec![Temp(0)],
                    },
                    poll(2),
                    poll(3),
                    Stmt::ConstInt(Temp(4), i64::from(DATA_BASE)),
                    Stmt::ConstInt(Temp(5), i64::from(DATA_BASE) + 8),
                    store(4, 2),
                    store(5, 3),
                ],
                ret_unit(),
            )],
        );
        let mut m = with_alloc(main);
        m.functions.push(addr);
        let out = valid(&m);
        assert!(out.imports.is_empty(), "{:?}", out.imports);
        let Some(mut node) = crate::test_node() else {
            return;
        };
        let dir = std::env::temp_dir().join("nova-codegen-wasm-yield");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        std::fs::write(dir.join("app.wasm"), &out.wasm).expect("the module");
        let script = format!(
            "const fs = require(\"fs\");\n\
             WebAssembly.instantiate(fs.readFileSync(process.argv[2]), {{ env: {{}} }}).then(({{ instance }}) => {{\n  \
             instance.exports.main();\n  \
             const view = new DataView(instance.exports.memory.buffer);\n  \
             console.log(view.getBigInt64({DATA_BASE}, true), view.getBigInt64({DATA_BASE} + 8, true));\n\
             }});\n"
        );
        std::fs::write(dir.join("run.cjs"), script).expect("the script");
        let out = node
            .arg(dir.join("run.cjs"))
            .arg(dir.join("app.wasm"))
            .output()
            .expect("node runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&out.stdout), "0n 1n\n");
    }

    /// A runtime function `std/rt` implements must have become a call to
    /// it; one WASM lacks must have been refused already (E0091).
    #[test]
    fn a_runtime_function_that_is_not_the_shims_is_refused() {
        for (func, says) in [
            (
                RtFunc::StrConcat,
                "MIR must call `std/rt`'s `rt_str_concat`",
            ),
            (RtFunc::TestSelector, "which has no such function"),
        ] {
            let f = |func| {
                func_with(
                    "main",
                    vec![block(
                        vec![Stmt::CallRuntime {
                            dst: None,
                            func,
                            args: vec![],
                        }],
                        ret_unit(),
                    )],
                )
            };
            let e = compile_module(&module(vec![f(func)])).expect_err("refused");
            assert!(e.to_string().contains(says), "{e}");
        }
    }

    fn func_with(name: &str, blocks: Vec<Block>) -> MirFn {
        func(name, 0, vec![], MirTy::Unit, blocks)
    }

    /// `%` on `Float`s is refused, as the native backends refuse it.
    #[test]
    fn float_remainder_is_refused() {
        let f = func(
            "main",
            0,
            vec![MirTy::F64],
            MirTy::Unit,
            vec![block(
                vec![
                    Stmt::ConstFloat(Temp(0), 1.0),
                    Stmt::Bin {
                        dst: Temp(0),
                        op: BinOp::Rem,
                        class: OperandClass::Float,
                        lhs: Temp(0),
                        rhs: Temp(0),
                    },
                ],
                ret_unit(),
            )],
        );
        let e = compile_module(&module(vec![f])).expect_err("refused");
        assert!(e.to_string().contains("float remainder (%)"), "{e}");
    }

    /// A bump allocator over the heap, as `std/rt`'s `rt_alloc` would be:
    /// `(size: i64) -> i64`. Its state word is at 8.
    fn alloc_fn() -> MirFn {
        let m = |op, args: Vec<u32>, dst: Option<u32>| Stmt::Mem {
            dst: dst.map(Temp),
            op,
            args: args.into_iter().map(Temp).collect(),
        };
        // t0 size, t1 state address, t2 at, t3 end
        func(
            "rt_alloc.9",
            1,
            vec![MirTy::I64, MirTy::I64, MirTy::I64, MirTy::I64],
            MirTy::I64,
            vec![block(
                vec![
                    Stmt::ConstInt(Temp(1), 8),
                    m(MemOp::Load64, vec![1], Some(2)),
                    Stmt::Bin {
                        dst: Temp(3),
                        op: BinOp::Add,
                        class: OperandClass::Int,
                        lhs: Temp(2),
                        rhs: Temp(0),
                    },
                    m(MemOp::Store64, vec![1, 3], None),
                ],
                Terminator::Return(Some(Temp(2))),
            )],
        )
    }

    fn with_alloc(main: MirFn) -> Module {
        Module {
            functions: vec![main, alloc_fn()],
            runtime_alloc: Some("rt_alloc.9".to_string()),
            ..Default::default()
        }
    }

    fn exports(wasm: &[u8]) -> Vec<String> {
        wasmparser::Parser::new(0)
            .parse_all(wasm)
            .filter_map(Result::ok)
            .filter_map(|p| match p {
                wasmparser::Payload::ExportSection(r) => Some(
                    r.into_iter()
                        .filter_map(Result::ok)
                        .map(|e| e.name.to_string())
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .flatten()
            .collect()
    }

    /// Every heap statement, through the module's allocator, which is
    /// exported as `nova_alloc` (spec §4.4, §4.7).
    #[test]
    fn heap_objects_lower_through_the_allocator() {
        // t0 sum, t1 tag, t2 int, t3 record, t4 array, t5 len, t6 float,
        // t7 bool
        let stmts = vec![
            Stmt::ConstInt(Temp(2), 5),
            Stmt::ConstFloat(Temp(6), 2.5),
            Stmt::ConstBool(Temp(7), true),
            Stmt::MakeSum {
                dst: Temp(0),
                tag: 1,
                fields: vec![(Temp(2), MirTy::I64), (Temp(6), MirTy::F64)],
            },
            Stmt::SumTag {
                dst: Temp(1),
                sum: Temp(0),
            },
            Stmt::SumField {
                dst: Temp(6),
                sum: Temp(0),
                index: 1,
                ty: MirTy::F64,
            },
            Stmt::MakeRecord {
                dst: Temp(3),
                fields: vec![(Temp(0), MirTy::Ptr), (Temp(7), MirTy::I8)],
            },
            Stmt::RecordField {
                dst: Temp(0),
                record: Temp(3),
                index: 0,
                ty: MirTy::Ptr,
            },
            Stmt::SetField {
                record: Temp(3),
                index: 1,
                value: Temp(7),
                ty: MirTy::I8,
            },
            Stmt::MakeArray {
                dst: Temp(4),
                elems: vec![(Temp(2), MirTy::I64)],
            },
            Stmt::ArrayLen {
                dst: Temp(5),
                arr: Temp(4),
            },
            Stmt::ArrayAlloc {
                dst: Temp(4),
                len: Temp(5),
            },
            Stmt::ArraySet {
                arr: Temp(4),
                index: Temp(1),
                value: Temp(2),
                ty: MirTy::I64,
            },
            Stmt::ArrayGet {
                dst: Temp(2),
                arr: Temp(4),
                index: Temp(1),
                ty: MirTy::I64,
            },
        ];
        let temps = vec![
            MirTy::Ptr,
            MirTy::I64,
            MirTy::I64,
            MirTy::Ptr,
            MirTy::Ptr,
            MirTy::I64,
            MirTy::F64,
            MirTy::I8,
        ];
        let out = valid(&with_alloc(func(
            "main",
            0,
            temps,
            MirTy::Unit,
            vec![block(stmts, ret_unit())],
        )));
        assert_eq!(exports(&out.wasm), ["main", "memory", "nova_alloc"]);
    }

    /// The other functions MIR names for the shim are exported under their
    /// names, after the allocator; one the module lacks is refused (spec
    /// 4.1 §6.3).
    #[test]
    fn the_runtime_exports_are_exported() {
        let main = func(
            "main",
            0,
            vec![],
            MirTy::Unit,
            vec![block(vec![], ret_unit())],
        );
        let mut m = with_alloc(main);
        m.runtime_exports = vec![("nova_alloc_leaf", "rt_alloc.9".to_string())];
        let out = valid(&m);
        assert_eq!(
            exports(&out.wasm),
            ["main", "memory", "nova_alloc", "nova_alloc_leaf"]
        );
        m.runtime_exports = vec![("nova_turn", "rt_turn.4".to_string())];
        let e = compile_module(&m).expect_err("refused");
        assert!(e.to_string().contains("`rt_turn.4`"), "{e}");
    }

    /// A string literal is a header in the data segment: an address, with no
    /// allocation (spec §4.5).
    #[test]
    fn a_string_literal_is_a_static_header() {
        let main = func(
            "main",
            0,
            vec![MirTy::Ptr],
            MirTy::Unit,
            vec![block(
                vec![
                    Stmt::ConstStr(Temp(0), "hey".to_string()),
                    Stmt::ConstStr(Temp(0), "hey".to_string()),
                ],
                ret_unit(),
            )],
        );
        let out = valid(&module(vec![main]));
        let data: Vec<(u32, Vec<u8>)> = wasmparser::Parser::new(0)
            .parse_all(&out.wasm)
            .filter_map(Result::ok)
            .filter_map(|p| match p {
                wasmparser::Payload::DataSection(r) => Some(
                    r.into_iter()
                        .filter_map(Result::ok)
                        .map(|d| (0, d.data.to_vec()))
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .flatten()
            .collect();
        // One literal, once: length 3, bytes at 128 + 16 = 144, then "hey",
        // padded to 8.
        let mut want = 3u64.to_le_bytes().to_vec();
        want.extend(144u32.to_le_bytes());
        want.extend([0; 4]);
        want.extend(b"hey\0\0\0\0\0");
        assert_eq!(data, [(0, want)]);
    }

    /// A function value is its table index and an environment; calling one
    /// is `call_indirect` (spec §4.3).
    #[test]
    fn a_closure_is_a_table_index_and_an_environment() {
        // The closure: (env, x) -> x + its capture.
        let mut lambda = func(
            "lambda.3",
            1,
            vec![MirTy::Ptr, MirTy::I64, MirTy::I64],
            MirTy::I64,
            vec![block(
                vec![
                    Stmt::RecordField {
                        dst: Temp(2),
                        record: Temp(0),
                        index: 0,
                        ty: MirTy::I64,
                    },
                    Stmt::Bin {
                        dst: Temp(1),
                        op: BinOp::Add,
                        class: OperandClass::Int,
                        lhs: Temp(1),
                        rhs: Temp(2),
                    },
                ],
                Terminator::Return(Some(Temp(1))),
            )],
        );
        lambda.takes_env = true;
        lambda.capture_count = 1;
        let main = func(
            "main",
            0,
            vec![MirTy::I64, MirTy::Ptr],
            MirTy::Unit,
            vec![block(
                vec![
                    Stmt::ConstInt(Temp(0), 2),
                    Stmt::MakeClosure {
                        dst: Temp(1),
                        code: "lambda.3".to_string(),
                        captures: vec![(Temp(0), MirTy::I64)],
                    },
                    Stmt::CallIndirect {
                        dst: Some(Temp(0)),
                        callee: Temp(1),
                        params: vec![MirTy::I64],
                        ret: MirTy::I64,
                        args: vec![Temp(0)],
                    },
                ],
                ret_unit(),
            )],
        );
        let mut m = with_alloc(main);
        m.functions.push(lambda);
        let out = valid(&m);
        let elements: usize = wasmparser::Parser::new(0)
            .parse_all(&out.wasm)
            .filter_map(Result::ok)
            .map(|p| match p {
                wasmparser::Payload::ElementSection(r) => r.count() as usize,
                _ => 0,
            })
            .sum();
        assert_eq!(elements, 1);
    }

    /// A call MIR routed to `std/rt` passes a `Ptr` to an `Int` parameter
    /// and keeps an `Int` result in a `Ptr`: the backend converts both, or
    /// the module would not validate (spec §5.4).
    #[test]
    fn a_call_into_std_rt_converts_between_ptr_and_int() {
        let rt = func(
            "rt_str_concat.4",
            1,
            vec![MirTy::I64],
            MirTy::I64,
            vec![block(vec![], Terminator::Return(Some(Temp(0))))],
        );
        let main = func(
            "main",
            0,
            vec![MirTy::Ptr],
            MirTy::Unit,
            vec![block(
                vec![
                    Stmt::ConstStr(Temp(0), "a".to_string()),
                    Stmt::Call {
                        dst: Some(Temp(0)),
                        callee: "rt_str_concat.4".to_string(),
                        args: vec![Temp(0)],
                    },
                ],
                ret_unit(),
            )],
        );
        valid(&module(vec![main, rt]));
    }

    /// A heap object needs the allocator; a module without one is refused.
    #[test]
    fn a_heap_object_without_an_allocator_is_refused() {
        let main = func(
            "main",
            0,
            vec![MirTy::Ptr],
            MirTy::Unit,
            vec![block(
                vec![Stmt::MakeRecord {
                    dst: Temp(0),
                    fields: vec![],
                }],
                ret_unit(),
            )],
        );
        let e = compile_module(&module(vec![main])).expect_err("refused");
        assert!(e.to_string().contains("no allocator"), "{e}");
    }

    trait CloneStmt {
        fn clone_stmt(&self) -> Stmt;
    }

    impl CloneStmt for Stmt {
        fn clone_stmt(&self) -> Stmt {
            match self {
                Stmt::Bin {
                    dst,
                    op,
                    class,
                    lhs,
                    rhs,
                } => Stmt::Bin {
                    dst: *dst,
                    op: *op,
                    class: *class,
                    lhs: *lhs,
                    rhs: *rhs,
                },
                other => panic!("not cloned: {other:?}"),
            }
        }
    }
}
