//! LLVM codegen backend for Nova — the optimizing release backend.
//!
//! Rather than link against `llvm-sys`/`inkwell` (which would require a
//! matching LLVM installed at build time), this backend emits **textual LLVM
//! IR** (`.ll`) from monomorphized MIR. `nova build --release` writes the IR
//! and hands it to a discovered LLVM toolchain (`clang`/`llc`) for optimizing
//! compilation, reusing the existing native linker path for the final
//! executable. The IR requires LLVM >= 15 (opaque pointers).
//!
//! The emitted code mirrors the Cranelift backend's semantics exactly — the
//! same 8-byte-slot heap layouts (sum `{ tag, fields… }`, record `{ fields… }`,
//! array `{ len, elems… }`, closure fat pointer `{ code, env }`), the same
//! env-first indirect-call ABI, and the same `nova_rt_*` runtime — so a
//! program behaves identically under `nova run`, debug `nova build`, and
//! `nova build --release`.
//!
//! MIR temps are not SSA (they are reassigned across blocks, e.g. loop
//! counters), so each temp becomes an entry-block `alloca` with `load`/`store`
//! at every use/def. LLVM's `mem2reg` promotes these to SSA registers with
//! phis during optimization, keeping this emitter simple and obviously correct.

use anyhow::{bail, Result};
use nova_hir::BinOp;
use nova_mir::{Function, MirTy, Module, OperandClass, RtFunc, Stmt, Temp, Terminator};
use std::collections::HashMap;
use std::fmt::Write as _;

/// The symbol the MIR `main` is emitted under, leaving the conventional `main`
/// name for the exported C entry wrapper (matching the Cranelift object path).
pub const NOVA_ENTRY_SYMBOL: &str = "nova_main";

/// The LLVM trap intrinsic used by `Terminator::Trap`. Not `RtFunc`-driven:
/// it's an LLVM builtin, not a Nova runtime symbol.
const TRAP_DECL: &str = "declare void @llvm.trap()";

/// The string constructor used by `ConstStr`, called directly by its raw
/// symbol (never through `Stmt::CallRuntime`), so it has no `RtFunc` variant
/// to generate a declaration from.
const STR_NEW_DECL: &str = "declare ptr @nova_rt_str_new(ptr, i64)";

/// The LLVM declaration for one `RtFunc`, generated from `symbol()` +
/// `signature()` rather than hand-written. This is the actual fix for the
/// bug class this backend used to be exposed to: previously `DECLS` was a
/// hand-copied string list with no compile-time tie to `RtFunc` at all, so
/// a new variant could be added and its declaration forgotten here — the
/// crate still compiled clean, and `CallRuntime`'s `call {ret} @{symbol}`
/// (driven directly by `RtFunc::signature`/`symbol`) would then reference an
/// undeclared function, which is invalid LLVM IR. Generating the
/// declaration from the same `signature()`/`symbol()` the call site uses
/// makes that impossible: the two can never disagree. See the `nova-mir`
/// module docs on `RtFunc::ALL` for how the variant list itself is kept
/// exhaustive.
fn rt_func_decl(rt: RtFunc) -> String {
    let (params, ret) = rt.signature();
    let params: Vec<&str> = params.iter().map(|&p| llty(p)).collect();
    format!(
        "declare {} @{}({})",
        llty(ret),
        rt.symbol(),
        params.join(", ")
    )
}

/// Compile a monomorphized MIR module to a textual LLVM IR module.
pub fn compile_ir(mir: &Module) -> Result<String> {
    if !mir.functions.iter().any(|f| f.name == "main") {
        bail!("no `main` function in MIR module");
    }
    // Return class of every function, so a call site can spell the call's
    // result type even when the result is discarded.
    let ret_of: HashMap<&str, MirTy> = mir
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.ret))
        .chain(mir.externs.iter().map(|e| (e.symbol.as_str(), e.ret)))
        .collect();

    let mut strings: Vec<String> = Vec::new();
    let mut string_index: HashMap<String, usize> = HashMap::new();
    let mut body = String::new();

    for f in &mir.functions {
        let mut fe = FnEmit {
            out: &mut body,
            f,
            ret_of: &ret_of,
            strings: &mut strings,
            string_index: &mut string_index,
            ssa: 0,
        };
        fe.emit()?;
        body.push('\n');
    }

    let mut out = String::new();
    out.push_str("; LLVM IR emitted by the Nova compiler (release backend).\n");
    out.push_str("; Requires LLVM >= 15 (opaque pointers).\n\n");
    out.push_str(TRAP_DECL);
    out.push('\n');
    for rt in RtFunc::ALL {
        out.push_str(&rt_func_decl(rt));
        out.push('\n');
    }
    out.push_str(STR_NEW_DECL);
    out.push('\n');
    // Declare each extern (FFI) symbol; resolved at run time by the system
    // linker against the C runtime (the call sites spell `@"symbol"`).
    for ext in &mir.externs {
        let params: Vec<&str> = ext.params.iter().map(|&t| llty(t)).collect();
        let _ = writeln!(
            out,
            "declare {} @\"{}\"({})",
            llty(ext.ret),
            ext.symbol,
            params.join(", ")
        );
    }
    out.push('\n');
    out.push_str(&body);

    // String literal globals (order-independent from their uses).
    for (i, s) in strings.iter().enumerate() {
        let bytes: Vec<u8> = if s.is_empty() {
            vec![0]
        } else {
            s.as_bytes().to_vec()
        };
        let _ = writeln!(
            out,
            "@str{i} = private unnamed_addr constant [{} x i8] c\"{}\", align 1",
            bytes.len(),
            escape_bytes(&bytes),
        );
    }
    out.push('\n');

    // Exported C entry point: `int main(int, char**) { nova_main(); return 0; }`.
    out.push_str("define i32 @main(i32 %argc, ptr %argv) {\nentry:\n");
    out.push_str("  call void @nova_main()\n  ret i32 0\n}\n");

    Ok(out)
}

/// The LLVM type spelling for a value class (`void` for unit).
fn llty(ty: MirTy) -> &'static str {
    match ty {
        MirTy::I64 => "i64",
        MirTy::F64 => "double",
        MirTy::I8 => "i8",
        MirTy::Ptr => "ptr",
        MirTy::Unit => "void",
    }
}

/// Escape bytes for an LLVM `c"..."` string constant.
fn escape_bytes(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len());
    for &b in bytes {
        // Printable ASCII except `"` and `\` may appear literally.
        if (0x20..=0x7e).contains(&b) && b != b'"' && b != b'\\' {
            s.push(b as char);
        } else {
            let _ = write!(s, "\\{b:02X}");
        }
    }
    s
}

/// A `double` constant as its exact IEEE-754 bit pattern (LLVM hex-float form),
/// avoiding any decimal round-tripping.
fn float_hex(v: f64) -> String {
    format!("0x{:016X}", v.to_bits())
}

/// Per-function IR emission state.
struct FnEmit<'a> {
    out: &'a mut String,
    f: &'a Function,
    ret_of: &'a HashMap<&'a str, MirTy>,
    strings: &'a mut Vec<String>,
    string_index: &'a mut HashMap<String, usize>,
    ssa: u32,
}

impl<'a> FnEmit<'a> {
    fn line(&mut self, s: impl AsRef<str>) {
        self.out.push_str(s.as_ref());
        self.out.push('\n');
    }

    /// A fresh, named SSA temporary (named values dodge LLVM's implicit
    /// sequential numbering rules for unnamed `%N` values).
    fn fresh(&mut self) -> String {
        let n = self.ssa;
        self.ssa += 1;
        format!("%r{n}")
    }

    fn temp_ty(&self, t: Temp) -> MirTy {
        self.f.temps[t.0 as usize]
    }

    fn slot(&self, t: Temp) -> String {
        format!("%t{}.slot", t.0)
    }

    /// Load a temp's current value into a fresh SSA name.
    fn load(&mut self, t: Temp) -> String {
        let ty = llty(self.temp_ty(t));
        let slot = self.slot(t);
        let r = self.fresh();
        self.line(format!("  {r} = load {ty}, ptr {slot}"));
        r
    }

    /// Store a value into a temp's slot (no-op for unit temps).
    fn store(&mut self, t: Temp, val: &str) {
        if self.temp_ty(t) == MirTy::Unit {
            return;
        }
        let ty = llty(self.temp_ty(t));
        let slot = self.slot(t);
        self.line(format!("  store {ty} {val}, ptr {slot}"));
    }

    /// Number of leading ABI parameter temps (real params + optional env ptr).
    fn abi_count(&self) -> usize {
        self.f.params as usize + if self.f.takes_env { 1 } else { 0 }
    }

    fn intern_string(&mut self, s: &str) -> usize {
        if let Some(&i) = self.string_index.get(s) {
            return i;
        }
        let i = self.strings.len();
        self.strings.push(s.to_string());
        self.string_index.insert(s.to_string(), i);
        i
    }

    fn emit(&mut self) -> Result<()> {
        let symbol = if self.f.name == "main" {
            NOVA_ENTRY_SYMBOL
        } else {
            self.f.name.as_str()
        };
        let ret = llty(self.f.ret);

        // Signature: non-unit ABI param temps become named LLVM parameters.
        let abi = self.abi_count();
        let mut params = String::new();
        for i in 0..abi {
            let t = Temp(i as u32);
            if self.temp_ty(t) == MirTy::Unit {
                continue;
            }
            if !params.is_empty() {
                params.push_str(", ");
            }
            let _ = write!(params, "{} %p{i}", llty(self.temp_ty(t)));
        }
        self.line(format!("define {ret} @\"{symbol}\"({params}) {{"));

        // Entry block: allocate a slot per non-unit temp, seed parameters, then
        // jump to the first MIR block (so `bb0` may itself be a loop target).
        self.line("entry:");
        let temp_count = self.f.temps.len();
        for i in 0..temp_count {
            let t = Temp(i as u32);
            if self.temp_ty(t) == MirTy::Unit {
                continue;
            }
            let ty = llty(self.temp_ty(t));
            let slot = self.slot(t);
            self.line(format!("  {slot} = alloca {ty}"));
        }
        for i in 0..abi {
            let t = Temp(i as u32);
            if self.temp_ty(t) == MirTy::Unit {
                continue;
            }
            let ty = llty(self.temp_ty(t));
            let slot = self.slot(t);
            self.line(format!("  store {ty} %p{i}, ptr {slot}"));
        }
        self.line("  br label %bb0");

        // One labeled block per MIR block. Copy the `&'a Function` reference
        // into a local so iterating its blocks (borrowing `'a` data) does not
        // conflict with mutating `self` during emission.
        let f: &'a Function = self.f;
        for (i, block) in f.blocks.iter().enumerate() {
            self.line(format!("bb{i}:"));
            for stmt in &block.stmts {
                self.stmt(stmt)?;
            }
            self.terminator(&block.term)?;
        }

        self.line("}");
        Ok(())
    }

    /// Emit a byte-offset GEP into a heap object, yielding a fresh address.
    fn gep_byte(&mut self, base: &str, byte_off: i64) -> String {
        let r = self.fresh();
        self.line(format!(
            "  {r} = getelementptr inbounds i8, ptr {base}, i64 {byte_off}"
        ));
        r
    }

    fn stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::ConstInt(t, v) => self.store(*t, &v.to_string()),
            Stmt::ConstFloat(t, v) => {
                let hex = float_hex(*v);
                self.store(*t, &hex);
            }
            Stmt::ConstBool(t, v) => self.store(*t, if *v { "1" } else { "0" }),
            Stmt::ConstUnit(_) => {}
            Stmt::ConstStr(t, s) => {
                let idx = self.intern_string(s);
                let n = if s.is_empty() { 1 } else { s.len() };
                let p = self.fresh();
                self.line(format!(
                    "  {p} = getelementptr inbounds [{n} x i8], ptr @str{idx}, i64 0, i64 0"
                ));
                let r = self.fresh();
                self.line(format!(
                    "  {r} = call ptr @nova_rt_str_new(ptr {p}, i64 {})",
                    s.len()
                ));
                self.store(*t, &r);
            }
            Stmt::Copy { dst, src } => {
                if self.temp_ty(*src) != MirTy::Unit {
                    let v = self.load(*src);
                    self.store(*dst, &v);
                }
            }
            Stmt::Bin {
                dst,
                op,
                class,
                lhs,
                rhs,
            } => {
                let l = self.load(*lhs);
                let r = self.load(*rhs);
                let opnd = llty(self.temp_ty(*lhs));
                let v = self.binop(*op, *class, opnd, &l, &r)?;
                self.store(*dst, &v);
            }
            Stmt::Neg { dst, class, src } => {
                let v = self.load(*src);
                let ty = llty(self.temp_ty(*src));
                let r = self.fresh();
                match class {
                    OperandClass::Float => self.line(format!("  {r} = fneg {ty} {v}")),
                    _ => self.line(format!("  {r} = sub {ty} 0, {v}")),
                }
                self.store(*dst, &r);
            }
            Stmt::Not { dst, src } => {
                let v = self.load(*src);
                let r = self.fresh();
                self.line(format!("  {r} = xor i8 {v}, 1"));
                self.store(*dst, &r);
            }
            Stmt::BitNot { dst, src } => {
                let v = self.load(*src);
                let r = self.fresh();
                self.line(format!("  {r} = xor i64 {v}, -1"));
                self.store(*dst, &r);
            }
            Stmt::Call { dst, callee, args } => {
                let ret = *self.ret_of.get(callee.as_str()).unwrap_or(&MirTy::Unit);
                let arglist = self.arg_list(args);
                self.emit_call(*dst, ret, &format!("@\"{callee}\""), &arglist);
            }
            Stmt::CallRuntime { dst, func, args } => {
                let (_, ret) = func.signature();
                let arglist = self.arg_list(args);
                let callee = format!("@{}", func.symbol());
                self.emit_call(*dst, ret, &callee, &arglist);
            }
            Stmt::CallIndirect {
                dst,
                callee,
                params: _,
                ret,
                args,
            } => {
                // `callee` is a fat pointer `{ code_ptr, env_ptr }`; call
                // `code_ptr(env_ptr, args...)` (env-first ABI).
                let fat = self.load(*callee);
                let code = self.fresh();
                self.line(format!("  {code} = load ptr, ptr {fat}"));
                let env_addr = self.gep_byte(&fat, 8);
                let env = self.fresh();
                self.line(format!("  {env} = load ptr, ptr {env_addr}"));
                let mut arglist = format!("ptr {env}");
                for a in args {
                    if self.temp_ty(*a) == MirTy::Unit {
                        continue;
                    }
                    let ty = llty(self.temp_ty(*a));
                    let v = self.load(*a);
                    let _ = write!(arglist, ", {ty} {v}");
                }
                self.emit_call(*dst, *ret, &code, &arglist);
            }
            Stmt::MakeClosure {
                dst,
                code,
                captures,
            } => {
                let env = if captures.is_empty() {
                    "null".to_string()
                } else {
                    let size = 8 * captures.len();
                    let env = self.fresh();
                    self.line(format!("  {env} = call ptr @nova_rt_alloc(i64 {size})"));
                    for (i, (cap, ty)) in captures.iter().enumerate() {
                        if *ty == MirTy::Unit {
                            continue;
                        }
                        let v = self.load(*cap);
                        let addr = self.gep_byte(&env, (8 * i) as i64);
                        self.line(format!("  store {} {v}, ptr {addr}", llty(*ty)));
                    }
                    env
                };
                let fat = self.fresh();
                self.line(format!("  {fat} = call ptr @nova_rt_alloc(i64 16)"));
                self.line(format!("  store ptr @\"{code}\", ptr {fat}"));
                let env_addr = self.gep_byte(&fat, 8);
                self.line(format!("  store ptr {env}, ptr {env_addr}"));
                self.store(*dst, &fat);
            }
            Stmt::MakeSum { dst, tag, fields } => {
                let size = 8 + 8 * fields.len();
                let ptr = self.fresh();
                self.line(format!("  {ptr} = call ptr @nova_rt_alloc(i64 {size})"));
                self.line(format!("  store i64 {tag}, ptr {ptr}"));
                for (i, (field, ty)) in fields.iter().enumerate() {
                    if *ty == MirTy::Unit {
                        continue;
                    }
                    let v = self.load(*field);
                    let addr = self.gep_byte(&ptr, (8 + 8 * i) as i64);
                    self.line(format!("  store {} {v}, ptr {addr}", llty(*ty)));
                }
                self.store(*dst, &ptr);
            }
            Stmt::SumTag { dst, sum } => {
                let p = self.load(*sum);
                let r = self.fresh();
                self.line(format!("  {r} = load i64, ptr {p}"));
                self.store(*dst, &r);
            }
            Stmt::SumField {
                dst,
                sum,
                index,
                ty,
            } => {
                if *ty == MirTy::Unit {
                    return Ok(());
                }
                let p = self.load(*sum);
                let addr = self.gep_byte(&p, (8 + 8 * index) as i64);
                let r = self.fresh();
                self.line(format!("  {r} = load {}, ptr {addr}", llty(*ty)));
                self.store(*dst, &r);
            }
            Stmt::MakeRecord { dst, fields } => {
                let size = 8 * fields.len().max(1);
                let ptr = self.fresh();
                self.line(format!("  {ptr} = call ptr @nova_rt_alloc(i64 {size})"));
                for (i, (field, ty)) in fields.iter().enumerate() {
                    if *ty == MirTy::Unit {
                        continue;
                    }
                    let v = self.load(*field);
                    let addr = self.gep_byte(&ptr, (8 * i) as i64);
                    self.line(format!("  store {} {v}, ptr {addr}", llty(*ty)));
                }
                self.store(*dst, &ptr);
            }
            Stmt::RecordField {
                dst,
                record,
                index,
                ty,
            } => {
                if *ty == MirTy::Unit {
                    return Ok(());
                }
                let p = self.load(*record);
                let addr = self.gep_byte(&p, (8 * index) as i64);
                let r = self.fresh();
                self.line(format!("  {r} = load {}, ptr {addr}", llty(*ty)));
                self.store(*dst, &r);
            }
            Stmt::SetField {
                record,
                index,
                value,
                ty,
            } => {
                // A unit-typed field has no machine representation to store.
                if *ty == MirTy::Unit {
                    return Ok(());
                }
                let v = self.load(*value);
                let p = self.load(*record);
                // The same `8 * index` offset `RecordField` loads from.
                let addr = self.gep_byte(&p, (8 * index) as i64);
                self.line(format!("  store {} {v}, ptr {addr}", llty(*ty)));
            }
            Stmt::MakeArray { dst, elems } => {
                let size = 8 + 8 * elems.len();
                let ptr = self.fresh();
                self.line(format!("  {ptr} = call ptr @nova_rt_alloc(i64 {size})"));
                self.line(format!("  store i64 {}, ptr {ptr}", elems.len()));
                for (i, (el, ty)) in elems.iter().enumerate() {
                    if *ty == MirTy::Unit {
                        continue;
                    }
                    let v = self.load(*el);
                    let addr = self.gep_byte(&ptr, (8 + 8 * i) as i64);
                    self.line(format!("  store {} {v}, ptr {addr}", llty(*ty)));
                }
                self.store(*dst, &ptr);
            }
            Stmt::ArrayLen { dst, arr } => {
                let p = self.load(*arr);
                let r = self.fresh();
                self.line(format!("  {r} = load i64, ptr {p}"));
                self.store(*dst, &r);
            }
            Stmt::ArrayGet {
                dst,
                arr,
                index,
                ty,
            } => {
                if *ty == MirTy::Unit {
                    return Ok(());
                }
                let addr = self.array_elem_addr(*arr, *index);
                let r = self.fresh();
                self.line(format!("  {r} = load {}, ptr {addr}", llty(*ty)));
                self.store(*dst, &r);
            }
            Stmt::ArraySet {
                arr,
                index,
                value,
                ty,
            } => {
                if *ty == MirTy::Unit {
                    return Ok(());
                }
                let v = self.load(*value);
                let addr = self.array_elem_addr(*arr, *index);
                self.line(format!("  store {} {v}, ptr {addr}", llty(*ty)));
            }
        }
        Ok(())
    }

    /// Address of `arr[index]`: `arr + 8 (len header) + index*8`.
    fn array_elem_addr(&mut self, arr: Temp, index: Temp) -> String {
        let base = self.load(arr);
        let idx = self.load(index);
        let scaled = self.fresh();
        self.line(format!("  {scaled} = mul i64 {idx}, 8"));
        let off = self.fresh();
        self.line(format!("  {off} = add i64 {scaled}, 8"));
        let addr = self.fresh();
        self.line(format!(
            "  {addr} = getelementptr inbounds i8, ptr {base}, i64 {off}"
        ));
        addr
    }

    /// Build a comma-separated `<ty> <val>` argument list, skipping unit temps.
    fn arg_list(&mut self, args: &[Temp]) -> String {
        let mut out = String::new();
        for a in args {
            if self.temp_ty(*a) == MirTy::Unit {
                continue;
            }
            let ty = llty(self.temp_ty(*a));
            let v = self.load(*a);
            if !out.is_empty() {
                out.push_str(", ");
            }
            let _ = write!(out, "{ty} {v}");
        }
        out
    }

    /// Emit a call, storing the result into `dst` when the callee returns a
    /// value; a void callee is a statement.
    fn emit_call(&mut self, dst: Option<Temp>, ret: MirTy, callee: &str, arglist: &str) {
        if ret == MirTy::Unit {
            self.line(format!("  call void {callee}({arglist})"));
        } else {
            let r = self.fresh();
            self.line(format!("  {r} = call {} {callee}({arglist})", llty(ret)));
            if let Some(dst) = dst {
                self.store(dst, &r);
            }
        }
    }

    fn binop(
        &mut self,
        op: BinOp,
        class: OperandClass,
        opnd: &str,
        l: &str,
        r: &str,
    ) -> Result<String> {
        use BinOp::*;
        // Comparisons produce an `i1`, widened to the `i8` bool representation.
        let cmp_pred = match (class, op) {
            (OperandClass::Float, Eq) => Some("fcmp oeq"),
            (OperandClass::Float, Ne) => Some("fcmp une"),
            (OperandClass::Float, Lt) => Some("fcmp olt"),
            (OperandClass::Float, Le) => Some("fcmp ole"),
            (OperandClass::Float, Gt) => Some("fcmp ogt"),
            (OperandClass::Float, Ge) => Some("fcmp oge"),
            (_, Eq) => Some("icmp eq"),
            (_, Ne) => Some("icmp ne"),
            (_, Lt) => Some("icmp slt"),
            (_, Le) => Some("icmp sle"),
            (_, Gt) => Some("icmp sgt"),
            (_, Ge) => Some("icmp sge"),
            _ => None,
        };
        if let Some(pred) = cmp_pred {
            let c = self.fresh();
            self.line(format!("  {c} = {pred} {opnd} {l}, {r}"));
            let z = self.fresh();
            self.line(format!("  {z} = zext i1 {c} to i8"));
            return Ok(z);
        }

        let instr = match (class, op) {
            (OperandClass::Float, Add) => "fadd",
            (OperandClass::Float, Sub) => "fsub",
            (OperandClass::Float, Mul) => "fmul",
            (OperandClass::Float, Div) => "fdiv",
            (OperandClass::Float, Rem) => {
                bail!("float remainder (%) is not supported by the LLVM backend")
            }
            (OperandClass::Float, _) => bail!("bitwise operators are not defined for Float"),
            (_, Add) => "add",
            (_, Sub) => "sub",
            (_, Mul) => "mul",
            (_, Div) => "sdiv",
            (_, Rem) => "srem",
            (_, BitAnd) => "and",
            (_, BitOr) => "or",
            (_, BitXor) => "xor",
            (_, Shl) => "shl",
            (_, Shr) => "ashr",
            (_, Eq | Ne | Lt | Le | Gt | Ge) => unreachable!("handled as comparison above"),
        };
        let d = self.fresh();
        self.line(format!("  {d} = {instr} {opnd} {l}, {r}"));
        Ok(d)
    }

    fn terminator(&mut self, term: &Terminator) -> Result<()> {
        match term {
            Terminator::Goto(b) => self.line(format!("  br label %bb{}", b.0)),
            Terminator::Branch { cond, then_, else_ } => {
                let c = self.load(*cond);
                let c1 = self.fresh();
                self.line(format!("  {c1} = icmp ne i8 {c}, 0"));
                self.line(format!(
                    "  br i1 {c1}, label %bb{}, label %bb{}",
                    then_.0, else_.0
                ));
            }
            Terminator::Switch {
                disc,
                arms,
                default,
            } => {
                // The discriminant carries its own integer class: a sum tag /
                // `Int` / `Char` switch is `i64`, but a `Bool` match switches
                // on the `i8` scrutinee directly. The condition and every case
                // constant must share that type.
                let ty = llty(self.temp_ty(*disc));
                let d = self.load(*disc);
                let mut table = String::new();
                for (value, block) in arms {
                    let _ = write!(table, " {ty} {value}, label %bb{}", block.0);
                }
                self.line(format!(
                    "  switch {ty} {d}, label %bb{} [{table} ]",
                    default.0
                ));
            }
            Terminator::Return(value) => match value {
                Some(v) if self.f.ret != MirTy::Unit => {
                    let val = self.load(*v);
                    self.line(format!("  ret {} {val}", llty(self.f.ret)));
                }
                _ => self.line("  ret void"),
            },
            Terminator::Trap => {
                self.line("  call void @llvm.trap()");
                self.line("  unreachable");
            }
        }
        Ok(())
    }
}
