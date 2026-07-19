//! Cranelift codegen backend for Nova (the fast debug backend).
//!
//! Translates monomorphized MIR into native code. Phase 1 exposes the
//! in-memory JIT path used by `nova run`; object-file emission for
//! `nova build --debug` follows once the LLVM release backend lands.
//!
//! MIR temps become Cranelift frontend `Variable`s, so the SSA builder
//! inserts block parameters (phis) at join points automatically. Unit-class
//! temps carry no runtime value and are never materialized.

use anyhow::{anyhow, Context, Result};
use cranelift::codegen::ir::{types, AbiParam, InstBuilder, MemFlags, Signature, Value};
use cranelift::codegen::isa::TargetIsa;
use cranelift::codegen::settings::{self, Configurable};
use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext, Switch, Variable};
use cranelift::prelude::{EntityRef, FloatCC, IntCC, TrapCode};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module as _};
use nova_mir::{MirTy, Module as MirModule, OperandClass, RtFunc, Stmt, Terminator};
use rustc_hash::FxHashMap;
use std::sync::Arc;

// `mir_ty` is re-exported for driver convenience.
pub use nova_mir::mangle;

/// A JIT-compiled Nova program, ready to run in-process.
pub struct CompiledProgram {
    /// Kept alive for the lifetime of the compiled code.
    _module: JITModule,
    main: *const u8,
}

impl CompiledProgram {
    /// Execute the program's `main` function.
    pub fn run(&self) {
        // SAFETY: `main` was compiled from a `fn main()` with no params and
        // unit return, and `_module` keeps its memory alive.
        let entry: extern "C" fn() = unsafe { std::mem::transmute(self.main) };
        entry();
    }
}

/// JIT-compile a MIR module and return the runnable program.
///
/// All `nova-runtime` symbols are registered with the JIT so compiled code
/// can call the runtime directly.
pub fn compile_jit(mir: &MirModule) -> Result<CompiledProgram> {
    let isa = native_isa()?;
    let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    for (name, ptr) in nova_runtime::symbols() {
        builder.symbol(name, ptr);
    }
    let mut module = JITModule::new(builder);

    let functions = {
        let mut cg = Codegen::new(&mut module);
        cg.declare_runtime()?;
        cg.declare_functions(mir)?;
        cg.define_functions(mir)?;
        cg.functions
    };

    module
        .finalize_definitions()
        .context("failed to finalize JIT code")?;

    let main_id = *functions
        .get("main")
        .ok_or_else(|| anyhow!("no `main` function in MIR module"))?;
    let main = module.get_finalized_function(main_id);

    Ok(CompiledProgram {
        _module: module,
        main,
    })
}

fn native_isa() -> Result<Arc<dyn TargetIsa>> {
    let mut flags = settings::builder();
    flags
        .set("use_colocated_libcalls", "false")
        .context("setting cranelift flags")?;
    flags
        .set("is_pic", "false")
        .context("setting cranelift flags")?;
    let isa_builder = cranelift_native::builder()
        .map_err(|e| anyhow!("host machine is not supported by cranelift: {e}"))?;
    isa_builder
        .finish(settings::Flags::new(flags))
        .context("building native ISA")
}

/// Per-module codegen state: declared function and data ids.
struct Codegen<'m> {
    module: &'m mut JITModule,
    functions: FxHashMap<String, FuncId>,
    runtime: FxHashMap<&'static str, FuncId>,
    strings: FxHashMap<String, DataId>,
}

const ALL_RT: [RtFunc; 9] = [
    RtFunc::Println,
    RtFunc::Print,
    RtFunc::StrConcat,
    RtFunc::IntToStr,
    RtFunc::FloatToStr,
    RtFunc::BoolToStr,
    RtFunc::CharToStr,
    RtFunc::StrEq,
    RtFunc::AllocSum,
];

impl<'m> Codegen<'m> {
    fn new(module: &'m mut JITModule) -> Self {
        Self {
            module,
            functions: FxHashMap::default(),
            runtime: FxHashMap::default(),
            strings: FxHashMap::default(),
        }
    }

    fn ptr_ty(&self) -> types::Type {
        self.module.target_config().pointer_type()
    }

    fn cl_ty(&self, ty: MirTy) -> Option<types::Type> {
        match ty {
            MirTy::I64 => Some(types::I64),
            MirTy::F64 => Some(types::F64),
            MirTy::I8 => Some(types::I8),
            MirTy::Ptr => Some(self.ptr_ty()),
            MirTy::Unit => None,
        }
    }

    fn make_signature(&self, params: &[MirTy], ret: MirTy) -> Signature {
        let mut sig = self.module.make_signature();
        for p in params {
            if let Some(t) = self.cl_ty(*p) {
                sig.params.push(AbiParam::new(t));
            }
        }
        if let Some(t) = self.cl_ty(ret) {
            sig.returns.push(AbiParam::new(t));
        }
        sig
    }

    fn declare_runtime(&mut self) -> Result<()> {
        for rt in ALL_RT {
            let (params, ret) = rt.signature();
            let sig = self.make_signature(&params, ret);
            let id = self
                .module
                .declare_function(rt.symbol(), Linkage::Import, &sig)
                .with_context(|| format!("declaring runtime fn {}", rt.symbol()))?;
            self.runtime.insert(rt.symbol(), id);
        }
        // String literal constructor (used by ConstStr only).
        let ptr = self.ptr_ty();
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(ptr));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(ptr));
        let id = self
            .module
            .declare_function("nova_rt_str_new", Linkage::Import, &sig)
            .context("declaring nova_rt_str_new")?;
        self.runtime.insert("nova_rt_str_new", id);
        Ok(())
    }

    fn declare_functions(&mut self, mir: &MirModule) -> Result<()> {
        for f in &mir.functions {
            let params: Vec<MirTy> = f.temps[..f.params as usize].to_vec();
            let sig = self.make_signature(&params, f.ret);
            let id = self
                .module
                .declare_function(&f.name, Linkage::Local, &sig)
                .with_context(|| format!("declaring `{}`", f.name))?;
            self.functions.insert(f.name.clone(), id);
        }
        Ok(())
    }

    fn string_data(&mut self, s: &str) -> Result<DataId> {
        if let Some(id) = self.strings.get(s) {
            return Ok(*id);
        }
        let name = format!("str{}", self.strings.len());
        let id = self
            .module
            .declare_data(&name, Linkage::Local, false, false)
            .context("declaring string data")?;
        let mut desc = DataDescription::new();
        // Empty data definitions are rejected; pad the empty string.
        let bytes: Box<[u8]> = if s.is_empty() {
            Box::new([0u8])
        } else {
            s.as_bytes().into()
        };
        desc.define(bytes);
        self.module
            .define_data(id, &desc)
            .context("defining string data")?;
        self.strings.insert(s.to_string(), id);
        Ok(id)
    }

    fn define_functions(&mut self, mir: &MirModule) -> Result<()> {
        let mut fb_ctx = FunctionBuilderContext::new();
        for f in &mir.functions {
            self.define_function(f, &mut fb_ctx)
                .with_context(|| format!("compiling `{}`", f.name))?;
        }
        Ok(())
    }

    fn define_function(
        &mut self,
        f: &nova_mir::Function,
        fb_ctx: &mut FunctionBuilderContext,
    ) -> Result<()> {
        let func_id = self.functions[&f.name];
        let params: Vec<MirTy> = f.temps[..f.params as usize].to_vec();
        let sig = self.make_signature(&params, f.ret);

        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;

        // Collect everything needed from `self` before borrowing mutably.
        let mut builder = FunctionBuilder::new(&mut ctx.func, fb_ctx);

        // Declare a variable per non-unit temp.
        let mut vars: Vec<Option<Variable>> = Vec::with_capacity(f.temps.len());
        for (i, ty) in f.temps.iter().enumerate() {
            match self.cl_ty(*ty) {
                Some(t) => {
                    let var = Variable::new(i);
                    builder.declare_var(var, t);
                    vars.push(Some(var));
                }
                None => vars.push(None),
            }
        }

        // One Cranelift block per MIR block.
        let cl_blocks: Vec<_> = f.blocks.iter().map(|_| builder.create_block()).collect();
        let entry = cl_blocks[0];
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);

        // Bind incoming parameters to their variables (unit params carry
        // no ABI value, so zip non-unit vars with the block params).
        let entry_params: Vec<Value> = builder.block_params(entry).to_vec();
        let param_vars = vars.iter().take(f.params as usize).copied().flatten();
        for (var, value) in param_vars.zip(entry_params) {
            builder.def_var(var, value);
        }

        let mut tr = Translator {
            cg: self,
            builder,
            vars,
            cl_blocks,
        };
        for (i, block) in f.blocks.iter().enumerate() {
            if i != 0 {
                tr.builder.switch_to_block(tr.cl_blocks[i]);
            }
            for stmt in &block.stmts {
                tr.stmt(stmt)?;
            }
            tr.terminator(&block.term, f.ret)?;
        }
        tr.builder.seal_all_blocks();
        tr.builder.finalize();

        self.module
            .define_function(func_id, &mut ctx)
            .with_context(|| format!("defining `{}`", f.name))?;
        self.module.clear_context(&mut ctx);
        Ok(())
    }
}

/// Per-function translation state.
struct Translator<'a, 'm> {
    cg: &'a mut Codegen<'m>,
    builder: FunctionBuilder<'a>,
    vars: Vec<Option<Variable>>,
    cl_blocks: Vec<cranelift::codegen::ir::Block>,
}

impl<'a, 'm> Translator<'a, 'm> {
    fn use_temp(&mut self, t: nova_mir::Temp) -> Result<Value> {
        let var = self.vars[t.0 as usize]
            .ok_or_else(|| anyhow!("attempted to read a unit temp %{}", t.0))?;
        Ok(self.builder.use_var(var))
    }

    fn def_temp(&mut self, t: nova_mir::Temp, v: Value) {
        if let Some(var) = self.vars[t.0 as usize] {
            self.builder.def_var(var, v);
        }
    }

    fn call_func_id(&mut self, id: FuncId, args: &[Value]) -> Result<Option<Value>> {
        let func_ref = self.cg.module.declare_func_in_func(id, self.builder.func);
        let inst = self.builder.ins().call(func_ref, args);
        let results = self.builder.inst_results(inst);
        Ok(results.first().copied())
    }

    fn rt(&self, name: &'static str) -> FuncId {
        self.cg.runtime[name]
    }

    fn stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::ConstInt(t, v) => {
                let val = self.builder.ins().iconst(types::I64, *v);
                self.def_temp(*t, val);
            }
            Stmt::ConstFloat(t, v) => {
                let val = self.builder.ins().f64const(*v);
                self.def_temp(*t, val);
            }
            Stmt::ConstBool(t, v) => {
                let val = self.builder.ins().iconst(types::I8, *v as i64);
                self.def_temp(*t, val);
            }
            Stmt::ConstUnit(_) => {}
            Stmt::ConstStr(t, s) => {
                let data_id = self.cg.string_data(s)?;
                let gv = self
                    .cg
                    .module
                    .declare_data_in_func(data_id, self.builder.func);
                let ptr_ty = self.cg.ptr_ty();
                let ptr = self.builder.ins().global_value(ptr_ty, gv);
                let len = self.builder.ins().iconst(types::I64, s.len() as i64);
                let id = self.rt("nova_rt_str_new");
                let result = self
                    .call_func_id(id, &[ptr, len])?
                    .ok_or_else(|| anyhow!("str_new returns a value"))?;
                self.def_temp(*t, result);
            }
            Stmt::Copy { dst, src } => {
                let v = self.use_temp(*src)?;
                self.def_temp(*dst, v);
            }
            Stmt::Bin {
                dst,
                op,
                class,
                lhs,
                rhs,
            } => {
                let l = self.use_temp(*lhs)?;
                let r = self.use_temp(*rhs)?;
                let v = self.binop(*op, *class, l, r)?;
                self.def_temp(*dst, v);
            }
            Stmt::Neg { dst, class, src } => {
                let v = self.use_temp(*src)?;
                let out = match class {
                    OperandClass::Float => self.builder.ins().fneg(v),
                    _ => self.builder.ins().ineg(v),
                };
                self.def_temp(*dst, out);
            }
            Stmt::Not { dst, src } => {
                let v = self.use_temp(*src)?;
                let out = self.builder.ins().bxor_imm(v, 1);
                self.def_temp(*dst, out);
            }
            Stmt::BitNot { dst, src } => {
                let v = self.use_temp(*src)?;
                let out = self.builder.ins().bnot(v);
                self.def_temp(*dst, out);
            }
            Stmt::Call { dst, callee, args } => {
                let id = *self
                    .cg
                    .functions
                    .get(callee)
                    .ok_or_else(|| anyhow!("call to undeclared function `{callee}`"))?;
                let arg_vals = self.arg_values(args)?;
                let result = self.call_func_id(id, &arg_vals)?;
                if let (Some(dst), Some(v)) = (dst, result) {
                    self.def_temp(*dst, v);
                }
            }
            Stmt::CallRuntime { dst, func, args } => {
                let id = self.rt(func.symbol());
                let arg_vals = self.arg_values(args)?;
                let result = self.call_func_id(id, &arg_vals)?;
                if let (Some(dst), Some(v)) = (dst, result) {
                    self.def_temp(*dst, v);
                }
            }
            Stmt::CallIndirect {
                dst,
                callee,
                params,
                ret,
                args,
            } => {
                let sig = self.cg.make_signature(params, *ret);
                let sig_ref = self.builder.import_signature(sig);
                let callee_val = self.use_temp(*callee)?;
                let arg_vals = self.arg_values(args)?;
                let inst = self
                    .builder
                    .ins()
                    .call_indirect(sig_ref, callee_val, &arg_vals);
                let result = self.builder.inst_results(inst).first().copied();
                if let (Some(dst), Some(v)) = (dst, result) {
                    self.def_temp(*dst, v);
                }
            }
            Stmt::FnAddr { dst, callee } => {
                let id = *self
                    .cg
                    .functions
                    .get(callee)
                    .ok_or_else(|| anyhow!("address of undeclared function `{callee}`"))?;
                let func_ref = self.cg.module.declare_func_in_func(id, self.builder.func);
                let ptr_ty = self.cg.ptr_ty();
                let addr = self.builder.ins().func_addr(ptr_ty, func_ref);
                self.def_temp(*dst, addr);
            }
            Stmt::MakeSum { dst, tag, fields } => {
                let size = 8 + 8 * fields.len() as i64;
                let size_val = self.builder.ins().iconst(types::I64, size);
                let id = self.rt("nova_rt_alloc_sum");
                let ptr = self
                    .call_func_id(id, &[size_val])?
                    .ok_or_else(|| anyhow!("alloc_sum returns a value"))?;
                let tag_val = self.builder.ins().iconst(types::I64, *tag as i64);
                self.builder
                    .ins()
                    .store(MemFlags::trusted(), tag_val, ptr, 0);
                for (i, (field, ty)) in fields.iter().enumerate() {
                    if *ty == MirTy::Unit {
                        continue;
                    }
                    let v = self.use_temp(*field)?;
                    let offset = (8 + 8 * i) as i32;
                    self.builder
                        .ins()
                        .store(MemFlags::trusted(), v, ptr, offset);
                }
                self.def_temp(*dst, ptr);
            }
            Stmt::SumTag { dst, sum } => {
                let ptr = self.use_temp(*sum)?;
                let tag = self
                    .builder
                    .ins()
                    .load(types::I64, MemFlags::trusted(), ptr, 0);
                self.def_temp(*dst, tag);
            }
            Stmt::SumField {
                dst,
                sum,
                index,
                ty,
            } => {
                let Some(cl_ty) = self.cg.cl_ty(*ty) else {
                    return Ok(());
                };
                let ptr = self.use_temp(*sum)?;
                let offset = (8 + 8 * index) as i32;
                let v = self
                    .builder
                    .ins()
                    .load(cl_ty, MemFlags::trusted(), ptr, offset);
                self.def_temp(*dst, v);
            }
        }
        Ok(())
    }

    /// Collect argument values, skipping unit temps (they carry no data).
    fn arg_values(&mut self, args: &[nova_mir::Temp]) -> Result<Vec<Value>> {
        let mut vals = Vec::with_capacity(args.len());
        for a in args {
            if self.vars[a.0 as usize].is_some() {
                vals.push(self.use_temp(*a)?);
            }
        }
        Ok(vals)
    }

    fn binop(
        &mut self,
        op: nova_hir::BinOp,
        class: OperandClass,
        l: Value,
        r: Value,
    ) -> Result<Value> {
        use nova_hir::BinOp::*;
        let ins = self.builder.ins();
        let v = match (class, op) {
            (OperandClass::Float, Add) => ins.fadd(l, r),
            (OperandClass::Float, Sub) => ins.fsub(l, r),
            (OperandClass::Float, Mul) => ins.fmul(l, r),
            (OperandClass::Float, Div) => ins.fdiv(l, r),
            (OperandClass::Float, Rem) => {
                return Err(anyhow!(
                    "float remainder (%) is not supported yet by the Cranelift backend"
                ))
            }
            (OperandClass::Float, Eq) => ins.fcmp(FloatCC::Equal, l, r),
            (OperandClass::Float, Ne) => ins.fcmp(FloatCC::NotEqual, l, r),
            (OperandClass::Float, Lt) => ins.fcmp(FloatCC::LessThan, l, r),
            (OperandClass::Float, Le) => ins.fcmp(FloatCC::LessThanOrEqual, l, r),
            (OperandClass::Float, Gt) => ins.fcmp(FloatCC::GreaterThan, l, r),
            (OperandClass::Float, Ge) => ins.fcmp(FloatCC::GreaterThanOrEqual, l, r),
            (OperandClass::Float, _) => {
                return Err(anyhow!("bitwise operators are not defined for Float"))
            }
            (_, Add) => ins.iadd(l, r),
            (_, Sub) => ins.isub(l, r),
            (_, Mul) => ins.imul(l, r),
            (_, Div) => ins.sdiv(l, r),
            (_, Rem) => ins.srem(l, r),
            (_, Eq) => ins.icmp(IntCC::Equal, l, r),
            (_, Ne) => ins.icmp(IntCC::NotEqual, l, r),
            (_, Lt) => ins.icmp(IntCC::SignedLessThan, l, r),
            (_, Le) => ins.icmp(IntCC::SignedLessThanOrEqual, l, r),
            (_, Gt) => ins.icmp(IntCC::SignedGreaterThan, l, r),
            (_, Ge) => ins.icmp(IntCC::SignedGreaterThanOrEqual, l, r),
            (_, BitAnd) => ins.band(l, r),
            (_, BitOr) => ins.bor(l, r),
            (_, BitXor) => ins.bxor(l, r),
            (_, Shl) => ins.ishl(l, r),
            (_, Shr) => ins.sshr(l, r),
        };
        Ok(v)
    }

    fn terminator(&mut self, term: &Terminator, ret: MirTy) -> Result<()> {
        match term {
            Terminator::Goto(b) => {
                let target = self.cl_blocks[b.0 as usize];
                self.builder.ins().jump(target, &[]);
            }
            Terminator::Branch { cond, then_, else_ } => {
                let c = self.use_temp(*cond)?;
                let t = self.cl_blocks[then_.0 as usize];
                let e = self.cl_blocks[else_.0 as usize];
                self.builder.ins().brif(c, t, &[], e, &[]);
            }
            Terminator::Switch {
                disc,
                arms,
                default,
            } => {
                let mut switch = Switch::new();
                for (value, block) in arms {
                    switch.set_entry(*value as u128, self.cl_blocks[block.0 as usize]);
                }
                let d = self.use_temp(*disc)?;
                let default_block = self.cl_blocks[default.0 as usize];
                switch.emit(&mut self.builder, d, default_block);
            }
            Terminator::Return(value) => match (value, ret) {
                (Some(v), r) if r != MirTy::Unit => {
                    let val = self.use_temp(*v)?;
                    self.builder.ins().return_(&[val]);
                }
                _ => {
                    self.builder.ins().return_(&[]);
                }
            },
            Terminator::Trap => {
                self.builder.ins().trap(TrapCode::UnreachableCodeReached);
            }
        }
        Ok(())
    }
}

// Re-export for the driver's convenience.
pub use nova_mir::lower_module;

#[cfg(test)]
mod tests {
    // End-to-end execution tests live in the nova-cli integration suite,
    // where stdout of compiled programs can be captured; compiling MIR here
    // and running it would print into the test harness output.
}
