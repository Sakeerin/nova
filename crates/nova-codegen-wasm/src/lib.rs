//! The WASM backend: one MIR module to one WASM module (spec
//! `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
//! §4). See ARCHITECTURE.md for the pipeline.
//!
//! Values map as `I64` → `i64`, `F64` → `f64`, `I8` → `i32` (0 or 1),
//! `Ptr` → `i32`, and `Unit` to no value; each non-`Unit` temp is a local.
//! Blocks are structured by [`structure`]'s analysis, or, for an
//! irreducible graph, run by a dispatch loop. Runtime functions are of three
//! kinds on WASM ([`nova_mir::WasmRuntime`]): `std/rt`'s, which MIR already
//! calls directly; the shim's, imported from `env`; and none.

mod structure;

use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use nova_mir::{
    BinOp, Function as MirFn, MemOp, MirTy, Module, OperandClass, RtFunc, Stmt, Temp, Terminator,
    WasmRuntime,
};
use wasm_encoder::{
    BlockType, CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection,
    ImportSection, Instruction as I, MemArg, MemorySection, MemoryType, TypeSection, ValType,
};

use structure::{Cfg, UNREACHED};

/// Memory's first size, in 64 KiB pages: 16 MB (`30-FRONTEND.md` §10).
pub const MEMORY_PAGES: u64 = 256;
/// Memory's largest size, in pages: 1 GB.
pub const MEMORY_MAX_PAGES: u64 = 16384;
/// Where the data segment starts (spec §4.1): `0..8` is never an object,
/// and `8..64` holds `std/rt`'s state words.
pub const DATA_BASE: u32 = 64;

/// What [`compile_module`] makes.
#[derive(Debug)]
pub struct Output {
    /// The module's bytes.
    pub wasm: Vec<u8>,
    /// The shim's functions it imports from `env`, by field name, in the
    /// order of the module's imports.
    pub imports: Vec<&'static str>,
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
}

/// Lower `mir` to a WASM module. `main` is exported, with the memory.
pub fn compile_module(mir: &Module) -> Result<Output> {
    let mut types = Types::default();
    let mut imports = ImportSection::new();
    let mut layout = Layout {
        funcs: HashMap::new(),
        rets: HashMap::new(),
        params: HashMap::new(),
        hosts: HashMap::new(),
        heap_base: i64::from(DATA_BASE),
    };

    // The host functions the program calls, in first-use order.
    let mut host_order: Vec<RtFunc> = Vec::new();
    for f in &mir.functions {
        for b in &f.blocks {
            for s in &b.stmts {
                if let Stmt::CallRuntime { func, .. } = s {
                    match func.on_wasm() {
                        WasmRuntime::Host(_) => {
                            if !host_order.contains(func) {
                                host_order.push(*func);
                            }
                        }
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
    let mut import_names = Vec::new();
    for (i, func) in host_order.iter().enumerate() {
        let WasmRuntime::Host(name) = func.on_wasm() else {
            unreachable!("only host functions are collected");
        };
        let (params, ret) = func.signature();
        let ty = types.of(vals(&params), val(ret).into_iter().collect());
        imports.import("env", name, EntityType::Function(ty));
        layout.hosts.insert(*func, i as u32);
        import_names.push(name);
    }
    let first = host_order.len() as u32;
    for (i, f) in mir.functions.iter().enumerate() {
        layout.funcs.insert(f.name.clone(), first + i as u32);
        layout.rets.insert(f.name.clone(), f.ret);
        layout.params.insert(f.name.clone(), abi_params(f).to_vec());
    }

    let mut functions = FunctionSection::new();
    let mut code = CodeSection::new();
    for f in &mir.functions {
        let ty = types.of(vals(abi_params(f)), val(f.ret).into_iter().collect());
        functions.function(ty);
        let body = FnLower::new(f, &layout)
            .lower()
            .map_err(|e| anyhow!("compiling `{}`: {e}", f.name))?;
        code.function(&body);
    }

    let mut module = wasm_encoder::Module::new();
    module.section(&types.section);
    module.section(&imports);
    module.section(&functions);
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
    module.section(&exports);
    module.section(&code);
    Ok(Output {
        wasm: module.finish(),
        imports: import_names,
    })
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
        decls.push((1, ValType::I32));
        FnLower {
            f,
            layout,
            locals,
            pc,
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
                for a in args {
                    self.get(*a);
                }
                self.out.push(I::Call(index));
                let ret = layout.rets[callee];
                self.result(*dst, ret);
            }
            Stmt::CallRuntime { dst, func, args } => {
                let index = *self
                    .layout
                    .hosts
                    .get(func)
                    .ok_or_else(|| anyhow!("`{}` is not a host function", func.symbol()))?;
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
            Stmt::Await { .. } => bail!(
                "an `.await` marker reached codegen: `nova-mir`'s async state-machine \
                 transform must split every `async fn` body at its await points first"
            ),
            other => bail!("not lowered yet: {other:?}"),
        }
        Ok(())
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
        }
        let produces = matches!(
            op,
            MemOp::Load8 | MemOp::Load64 | MemOp::Pages | MemOp::Grow | MemOp::HeapBase
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

    /// Compile and validate, returning the module's bytes.
    fn valid(m: &Module) -> Output {
        let out = compile_module(m).expect("compiles");
        wasmparser::Validator::new()
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

    /// A runtime function `std/rt` implements must have become a call to
    /// it; one WASM lacks must have been refused already (E0091).
    #[test]
    fn a_runtime_function_that_is_not_the_shims_is_refused() {
        for (func, says) in [
            (
                RtFunc::StrConcat,
                "MIR must call `std/rt`'s `rt_str_concat`",
            ),
            (RtFunc::TaskBlockOn, "which has no such function"),
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
