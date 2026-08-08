//! Cranelift codegen backend for Nova (the fast debug backend).
//!
//! Translates monomorphized MIR into native code, through two paths:
//!
//! - [`compile_jit`]: in-memory JIT for `nova run` — runtime symbols are
//!   registered directly with the JIT.
//! - [`compile_object`]: native object bytes for `nova build` — the MIR
//!   entry is emitted as `nova_main` plus an exported C `main(argc, argv)`
//!   wrapper, and runtime symbols resolve at link time against the
//!   `nova-runtime` static library.
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
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module as ClModule};
use cranelift_object::{ObjectBuilder, ObjectModule};
use nova_mir::{MirTy, Module as MirModule, OperandClass, RtFunc, Stmt, Terminator};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// The symbol name the MIR `main` gets in object files, so the exported
/// C-ABI `main(argc, argv)` wrapper can keep the conventional name.
pub const NOVA_ENTRY_SYMBOL: &str = "nova_main";

// `mir_ty` is re-exported for driver convenience.
pub use nova_mir::mangle;

/// An `extern` symbol the JIT could not resolve at run time. This is a user
/// error (a bad FFI declaration), not a compiler bug, so the driver reports it
/// as a clean diagnostic rather than an internal error.
#[derive(Debug)]
pub struct UnresolvedExternSymbol(pub String);

impl std::fmt::Display for UnresolvedExternSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot resolve external symbol `{}`", self.0)
    }
}

impl std::error::Error for UnresolvedExternSymbol {}

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
    let isa = native_isa(false)?;
    let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    for (name, ptr) in nova_runtime::symbols() {
        builder.symbol(name, ptr);
    }
    let mut module = JITModule::new(builder);

    let functions = {
        let mut cg = Codegen::new(&mut module);
        cg.declare_runtime()?;
        cg.declare_externs(mir)?;
        cg.declare_functions(mir, None)?;
        cg.define_functions(mir)?;
        cg.functions
    };

    // Imported extern symbols are resolved *inside* `finalize_definitions`,
    // where cranelift-jit `panic!`s on a symbol it cannot resolve rather than
    // returning an `Err`. Catch that unwind (silencing the default hook so no
    // raw stack trace prints) and turn it into a clean error, so an
    // unresolvable `extern` becomes a diagnostic instead of a compiler crash —
    // matching the graceful `nova build` linker-error path.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    // Map the (large) module error to a String inside the closure so the caught
    // value stays small (clippy::result_large_err) and is unwind-safe.
    let finalize = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        module.finalize_definitions().map_err(|e| e.to_string())
    }));
    std::panic::set_hook(prev_hook);
    match finalize {
        Ok(Ok(())) => {}
        Ok(Err(msg)) => return Err(anyhow!("failed to finalize JIT code: {msg}")),
        Err(payload) => {
            let detail = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("unknown error");
            // cranelift-jit panics as "can't resolve symbol <name>"; recover the
            // symbol so the driver can render a clean, user-facing diagnostic.
            let symbol = detail
                .strip_prefix("can't resolve symbol ")
                .unwrap_or(detail)
                .trim()
                .to_string();
            return Err(anyhow::Error::new(UnresolvedExternSymbol(symbol)));
        }
    }

    let main_id = *functions
        .get("main")
        .ok_or_else(|| anyhow!("no `main` function in MIR module"))?;
    let main = module.get_finalized_function(main_id);

    Ok(CompiledProgram {
        _module: module,
        main,
    })
}

/// Compile a MIR module to native object-file bytes for `nova build`.
///
/// The object exports a C `main(argc, argv) -> i32` that calls the Nova
/// entry ([`NOVA_ENTRY_SYMBOL`]); `nova_rt_*` symbols are unresolved
/// imports satisfied by linking the `nova-runtime` static library.
pub fn compile_object(mir: &MirModule) -> Result<Vec<u8>> {
    // PIC on Unix-likes (PIE-by-default linkers); non-PIC COFF on Windows.
    let isa = native_isa(!cfg!(windows))?;
    let obj_builder = ObjectBuilder::new(isa, "nova", cranelift_module::default_libcall_names())
        .context("creating object builder")?;
    let mut module = ObjectModule::new(obj_builder);

    {
        let mut cg = Codegen::new(&mut module);
        cg.declare_runtime()?;
        cg.declare_externs(mir)?;
        cg.declare_functions(mir, Some(NOVA_ENTRY_SYMBOL))?;
        cg.define_functions(mir)?;
        cg.emit_c_main()?;
    }

    let product = module.finish();
    product.emit().context("emitting object file")
}

fn native_isa(pic: bool) -> Result<Arc<dyn TargetIsa>> {
    let mut flags = settings::builder();
    flags
        .set("use_colocated_libcalls", "false")
        .context("setting cranelift flags")?;
    flags
        .set("is_pic", if pic { "true" } else { "false" })
        .context("setting cranelift flags")?;
    let isa_builder = cranelift_native::builder()
        .map_err(|e| anyhow!("host machine is not supported by cranelift: {e}"))?;
    isa_builder
        .finish(settings::Flags::new(flags))
        .context("building native ISA")
}

/// Per-module codegen state: declared function and data ids. Generic over
/// the Cranelift module flavor (JIT vs object emission).
struct Codegen<'m, M: ClModule> {
    module: &'m mut M,
    functions: FxHashMap<String, FuncId>,
    runtime: FxHashMap<&'static str, FuncId>,
    strings: FxHashMap<String, DataId>,
}

impl<'m, M: ClModule> Codegen<'m, M> {
    fn new(module: &'m mut M) -> Self {
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

    /// Number of leading ABI parameter temps: the real params plus a
    /// leading environment pointer for function values (closures/wrappers).
    fn abi_param_count(f: &nova_mir::Function) -> usize {
        f.params as usize + if f.takes_env { 1 } else { 0 }
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
        for rt in RtFunc::ALL {
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

    /// Declare each `extern` (FFI) symbol as an imported function, registered
    /// by its raw name so a `Stmt::Call` to that name resolves. The symbol is
    /// satisfied at run time by the JIT's dlsym fallback (`nova run`) or by the
    /// system linker against the C runtime (`nova build`).
    fn declare_externs(&mut self, mir: &MirModule) -> Result<()> {
        for ext in &mir.externs {
            let sig = self.make_signature(&ext.params, ext.ret);
            let id = self
                .module
                .declare_function(&ext.symbol, Linkage::Import, &sig)
                .with_context(|| format!("declaring extern `{}`", ext.symbol))?;
            self.functions.insert(ext.symbol.clone(), id);
        }
        Ok(())
    }

    /// Declare all MIR functions. With `rename_main`, the MIR entry is
    /// declared under that symbol (object mode, where the conventional
    /// `main` symbol is taken by the C wrapper); the lookup key stays
    /// `"main"` either way.
    fn declare_functions(&mut self, mir: &MirModule, rename_main: Option<&str>) -> Result<()> {
        for f in &mir.functions {
            let params: Vec<MirTy> = f.temps[..Self::abi_param_count(f)].to_vec();
            let sig = self.make_signature(&params, f.ret);
            let symbol = match rename_main {
                Some(entry) if f.name == "main" => entry,
                _ => f.name.as_str(),
            };
            let id = self
                .module
                .declare_function(symbol, Linkage::Local, &sig)
                .with_context(|| format!("declaring `{}`", f.name))?;
            self.functions.insert(f.name.clone(), id);
        }
        Ok(())
    }

    /// Emit the exported C entry point for object files:
    /// `main(argc: i32, argv: ptr) -> i32 { nova_main(); return 0 }`.
    fn emit_c_main(&mut self) -> Result<()> {
        let ptr = self.ptr_ty();
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I32));
        sig.params.push(AbiParam::new(ptr));
        sig.returns.push(AbiParam::new(types::I32));
        let main_id = self
            .module
            .declare_function("main", Linkage::Export, &sig)
            .context("declaring C main wrapper")?;

        let nova_main = *self
            .functions
            .get("main")
            .ok_or_else(|| anyhow!("no `main` function in MIR module"))?;

        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;
        let mut fb_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx);
        let block = builder.create_block();
        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);
        let func_ref = self.module.declare_func_in_func(nova_main, builder.func);
        builder.ins().call(func_ref, &[]);
        let zero = builder.ins().iconst(types::I32, 0);
        builder.ins().return_(&[zero]);
        builder.seal_all_blocks();
        builder.finalize();

        self.module
            .define_function(main_id, &mut ctx)
            .context("defining C main wrapper")?;
        self.module.clear_context(&mut ctx);
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
        let abi_count = Self::abi_param_count(f);
        let params: Vec<MirTy> = f.temps[..abi_count].to_vec();
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
        let param_vars = vars.iter().take(abi_count).copied().flatten();
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
struct Translator<'a, 'm, M: ClModule> {
    cg: &'a mut Codegen<'m, M>,
    builder: FunctionBuilder<'a>,
    vars: Vec<Option<Variable>>,
    cl_blocks: Vec<cranelift::codegen::ir::Block>,
}

impl<'a, 'm, M: ClModule> Translator<'a, 'm, M> {
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

    /// Address of array element `index`: `arr + 8 (len header) + index * 8`
    /// (all values occupy 8-byte slots). Assumes a 64-bit pointer target.
    fn array_elem_addr(&mut self, arr: nova_mir::Temp, index: nova_mir::Temp) -> Result<Value> {
        let base = self.use_temp(arr)?;
        let idx = self.use_temp(index)?;
        let byte_off = self.builder.ins().imul_imm(idx, 8);
        let elem = self.builder.ins().iadd(base, byte_off);
        Ok(self.builder.ins().iadd_imm(elem, 8))
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
                // `callee` is a fat pointer `{ code_ptr, env_ptr }`. Load both
                // and call `code_ptr(env_ptr, args...)`.
                let ptr_ty = self.cg.ptr_ty();
                let fat = self.use_temp(*callee)?;
                let code = self.builder.ins().load(ptr_ty, MemFlags::trusted(), fat, 0);
                let env = self.builder.ins().load(ptr_ty, MemFlags::trusted(), fat, 8);
                // Signature: (env, params...) -> ret.
                let mut sig_params = vec![MirTy::Ptr];
                sig_params.extend_from_slice(params);
                let sig = self.cg.make_signature(&sig_params, *ret);
                let sig_ref = self.builder.import_signature(sig);
                let mut arg_vals = vec![env];
                arg_vals.extend(self.arg_values(args)?);
                let inst = self.builder.ins().call_indirect(sig_ref, code, &arg_vals);
                let result = self.builder.inst_results(inst).first().copied();
                if let (Some(dst), Some(v)) = (dst, result) {
                    self.def_temp(*dst, v);
                }
            }
            Stmt::MakeClosure {
                dst,
                code,
                captures,
            } => {
                let ptr_ty = self.cg.ptr_ty();
                // Environment record of captured values (null when empty).
                let env = if captures.is_empty() {
                    self.builder.ins().iconst(ptr_ty, 0)
                } else {
                    let size = (8 * captures.len()) as i64;
                    let size_val = self.builder.ins().iconst(types::I64, size);
                    let alloc = self.rt("nova_rt_alloc");
                    let env = self
                        .call_func_id(alloc, &[size_val])?
                        .ok_or_else(|| anyhow!("alloc returns a value"))?;
                    for (i, (cap, ty)) in captures.iter().enumerate() {
                        if *ty == MirTy::Unit {
                            continue;
                        }
                        let v = self.use_temp(*cap)?;
                        self.builder
                            .ins()
                            .store(MemFlags::trusted(), v, env, (8 * i) as i32);
                    }
                    env
                };
                // Fat pointer `{ code_ptr, env_ptr }`.
                let size_val = self.builder.ins().iconst(types::I64, 16);
                let alloc = self.rt("nova_rt_alloc");
                let fat = self
                    .call_func_id(alloc, &[size_val])?
                    .ok_or_else(|| anyhow!("alloc returns a value"))?;
                let id = *self
                    .cg
                    .functions
                    .get(code)
                    .ok_or_else(|| anyhow!("closure code `{code}` is undeclared"))?;
                let func_ref = self.cg.module.declare_func_in_func(id, self.builder.func);
                let code_ptr = self.builder.ins().func_addr(ptr_ty, func_ref);
                self.builder
                    .ins()
                    .store(MemFlags::trusted(), code_ptr, fat, 0);
                self.builder.ins().store(MemFlags::trusted(), env, fat, 8);
                self.def_temp(*dst, fat);
            }
            Stmt::MakeSum { dst, tag, fields } => {
                let size = 8 + 8 * fields.len() as i64;
                let size_val = self.builder.ins().iconst(types::I64, size);
                let id = self.rt("nova_rt_alloc");
                let ptr = self
                    .call_func_id(id, &[size_val])?
                    .ok_or_else(|| anyhow!("alloc returns a value"))?;
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
            Stmt::MakeRecord { dst, fields } => {
                // Records have no tag: fields sit at offsets 8*i.
                let size = (8 * fields.len().max(1)) as i64;
                let size_val = self.builder.ins().iconst(types::I64, size);
                let id = self.rt("nova_rt_alloc");
                let ptr = self
                    .call_func_id(id, &[size_val])?
                    .ok_or_else(|| anyhow!("alloc returns a value"))?;
                for (i, (field, ty)) in fields.iter().enumerate() {
                    if *ty == MirTy::Unit {
                        continue;
                    }
                    let v = self.use_temp(*field)?;
                    let offset = (8 * i) as i32;
                    self.builder
                        .ins()
                        .store(MemFlags::trusted(), v, ptr, offset);
                }
                self.def_temp(*dst, ptr);
            }
            Stmt::RecordField {
                dst,
                record,
                index,
                ty,
            } => {
                let Some(cl_ty) = self.cg.cl_ty(*ty) else {
                    return Ok(());
                };
                let ptr = self.use_temp(*record)?;
                let offset = (8 * index) as i32;
                let v = self
                    .builder
                    .ins()
                    .load(cl_ty, MemFlags::trusted(), ptr, offset);
                self.def_temp(*dst, v);
            }
            Stmt::SetField {
                record,
                index,
                value,
                ty,
            } => {
                // A unit-typed field has no machine representation to store.
                if self.cg.cl_ty(*ty).is_none() {
                    return Ok(());
                }
                let ptr = self.use_temp(*record)?;
                let v = self.use_temp(*value)?;
                // The same `8 * index` offset `RecordField` loads from.
                let offset = (8 * index) as i32;
                self.builder
                    .ins()
                    .store(MemFlags::trusted(), v, ptr, offset);
            }
            Stmt::MakeArray { dst, elems } => {
                // Layout: { len: i64, elem0, elem1, ... } with 8-byte slots.
                let size = (8 + 8 * elems.len()) as i64;
                let size_val = self.builder.ins().iconst(types::I64, size);
                let alloc = self.rt("nova_rt_alloc");
                let ptr = self
                    .call_func_id(alloc, &[size_val])?
                    .ok_or_else(|| anyhow!("alloc returns a value"))?;
                let len_val = self.builder.ins().iconst(types::I64, elems.len() as i64);
                self.builder
                    .ins()
                    .store(MemFlags::trusted(), len_val, ptr, 0);
                for (i, (el, ty)) in elems.iter().enumerate() {
                    if *ty == MirTy::Unit {
                        continue;
                    }
                    let v = self.use_temp(*el)?;
                    let offset = (8 + 8 * i) as i32;
                    self.builder
                        .ins()
                        .store(MemFlags::trusted(), v, ptr, offset);
                }
                self.def_temp(*dst, ptr);
            }
            Stmt::ArrayAlloc { dst, len } => {
                // Same layout as `MakeArray`, but the element count is only
                // known at runtime: `8 + 8*len` bytes with `len` at offset 0.
                // The MIR lowering has already guaranteed `len >= 0`, and the
                // allocator zeroes, so the elements start valid and are then
                // filled by the lowering's own loop.
                let n = self.use_temp(*len)?;
                let eight = self.builder.ins().iconst(types::I64, 8);
                let bytes = self.builder.ins().imul(n, eight);
                let size = self.builder.ins().iadd(bytes, eight);
                let alloc = self.rt("nova_rt_alloc");
                let ptr = self
                    .call_func_id(alloc, &[size])?
                    .ok_or_else(|| anyhow!("alloc returns a value"))?;
                self.builder.ins().store(MemFlags::trusted(), n, ptr, 0);
                self.def_temp(*dst, ptr);
            }
            Stmt::ArrayLen { dst, arr } => {
                let ptr = self.use_temp(*arr)?;
                let len = self
                    .builder
                    .ins()
                    .load(types::I64, MemFlags::trusted(), ptr, 0);
                self.def_temp(*dst, len);
            }
            Stmt::ArrayGet {
                dst,
                arr,
                index,
                ty,
            } => {
                let Some(cl_ty) = self.cg.cl_ty(*ty) else {
                    return Ok(());
                };
                let addr = self.array_elem_addr(*arr, *index)?;
                let v = self.builder.ins().load(cl_ty, MemFlags::trusted(), addr, 0);
                self.def_temp(*dst, v);
            }
            Stmt::ArraySet {
                arr,
                index,
                value,
                ty,
            } => {
                if self.cg.cl_ty(*ty).is_none() {
                    return Ok(());
                }
                let addr = self.array_elem_addr(*arr, *index)?;
                let v = self.use_temp(*value)?;
                self.builder.ins().store(MemFlags::trusted(), v, addr, 0);
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

    use nova_mir::RtFunc;

    /// Every `RtFunc` must have a matching entry in `nova_runtime::symbols()`.
    ///
    /// This crate is the only place the containment can be checked: `nova-mir`
    /// must not depend on `nova-runtime` (the poll ABI and state layout are
    /// deliberately declared twice for that reason), and `nova-runtime` does
    /// not depend on `nova-mir`. This crate depends on both.
    ///
    /// The gap this closes is real, not hypothetical: the `rt_funcs!` macro
    /// guarantees a variant is declared to *both codegen backends*, because
    /// `RtFunc::ALL` is generated from the same identifier list as the enum.
    /// It guarantees nothing about `symbols()`, which is a separate,
    /// hand-maintained table. `compile_jit` builds the JIT's resolver from
    /// `symbols()` while `declare_runtime` imports all of `RtFunc::ALL`, and
    /// `cranelift-jit` resolves imports inside `finalize_definitions`, where
    /// an unresolvable symbol is a `panic!` rather than an `Err`. A variant
    /// added to `rt_funcs!` and forgotten here therefore compiles clean,
    /// passes every existing test, and fails the first time compiled code
    /// actually calls it. The `nova_rt_task_*` entries shipped in exactly that
    /// state, which is what this test was added for.
    ///
    /// Names, not addresses: `symbols()` returns `*const u8` function
    /// pointers, and comparing those would only restate that the same `as`
    /// cast appears twice. The symbol string is what the linker and the JIT
    /// actually match on.
    #[test]
    fn every_rt_func_symbol_is_registered_with_the_jit() {
        let registered: Vec<&str> = nova_runtime::symbols()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        let missing: Vec<&str> = RtFunc::ALL
            .iter()
            .map(|f| f.symbol())
            .filter(|s| !registered.contains(s))
            .collect();
        assert!(
            missing.is_empty(),
            "RtFunc variants with no nova_runtime::symbols() entry: {missing:?}. \
             Compiled code calling one of these makes cranelift-jit panic in \
             finalize_definitions; add it to symbols() in nova-runtime/src/lib.rs."
        );
    }

    /// The reverse direction is deliberately *not* asserted as equality:
    /// `symbols()` legitimately carries entries no `RtFunc` names, because
    /// codegen reaches some runtime functions without going through the enum
    /// (`nova_rt_str_new` is declared directly by `declare_runtime` for
    /// `ConstStr`). What must hold is that every registered name is a real,
    /// distinct symbol -- a duplicate entry would mean one registration
    /// silently shadowing another.
    #[test]
    fn registered_runtime_symbols_are_distinct() {
        let mut names: Vec<&str> = nova_runtime::symbols()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            before,
            "nova_runtime::symbols() has a duplicate entry"
        );
    }
}
