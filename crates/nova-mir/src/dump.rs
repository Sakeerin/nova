//! The MIR dump: what monomorphization, lowering and the async transform
//! built natively for one program, as text, one entry per line, which the
//! self-hosted compiler (`selfhost/`) prints too, so that the two can be
//! compared byte for byte (spec
//! `docs/superpowers/specs/2026-11-14-phase-5-4-mir-design.md` §4; ADR 0057
//! writes the grammar out, with an example of every form). It carries no
//! stability promise: it changes whenever MIR does, on both sides in one
//! pull request. `nova check --dump-mir FILE…` prints a program's part
//! after a `== FILE` header, and `--before-async` the module as lowering
//! left it, before the async transform (spec §3.1).
//!
//! - It follows the earlier dumps' conventions (ADR 0053 §1): a line is its
//!   indentation, two spaces per depth, `(` and its kind, its values and its
//!   fields `key=value`; its children follow, one level deeper, and its `)`
//!   closes it at the end of its last line. Strings and symbols are quoted
//!   as the AST dump quotes strings; diagnostics print as the resolution
//!   dump's, MIR's alone.
//! - **The first line says how far the root got:** `(Mir runs=false)` for
//!   a package module `check` does not lower; `checked=false` for a
//!   program that stopped before MIR, with no diagnostic, which the HIR
//!   dump prints; `lowered=false` for MIR's diagnostics; `lowered=true`,
//!   `async_lowered=` whether the transform ran, the counts, then the
//!   externs and every function.
//! - **Order is MIR's own,** nothing sorted: the functions as the worklist
//!   made them, the shim after them, the transform's wrappers after all;
//!   the externs as collected; temps and blocks by index.
//! - A statement is one line: its [`Stmt`] variant's name, then every field
//!   in declaration order: `_` for an absent destination, a temp `t<n>`, a
//!   block `bb<n>`, an integer in decimal, a float as the 16 lowercase hex
//!   digits of `to_bits`, a string or a symbol quoted, an operator, a
//!   class, a runtime function or a memory operation by its variant's name,
//!   a list in parentheses, each capture's, field's or element's class
//!   beside its temp (`t3:Ptr`), each switch arm's value beside its block
//!   (`0:bb2`).
//! - **Symbols print raw,** with absolute `DefId`s (`Vec_T.push.147`): MIR
//!   is monomorphized per program from `main`, so it has no std part.

use std::fmt::Write;

use nova_diagnostics::Diagnostic;
use nova_resolver::dump::{quote, write_diagnostics, Labels};

use crate::{
    BlockId, ExternAbi, Function, MemOp, MirTy, Module, OperandClass, Stmt, Temp, Terminator,
};

/// Every form the dump prints, as [`nova_resolver::dump::forms_on`] reads
/// a line, beyond the fields whose values are numbers: the harness checks
/// that the native dumps of the corpus and the cases print each of them but
/// [`NOT_NATIVE`]'s and print nothing unlisted
/// (`crates/nova-cli/tests/selfhost.rs`).
pub const KINDS: &[&str] = &[
    "(Mir",
    "(Mir runs=true",
    "(Mir runs=false",
    "(Mir checked=true",
    "(Mir checked=false",
    "(Mir lowered=true",
    "(Mir lowered=false",
    "(Mir async_lowered=true",
    "(Mir async_lowered=false",
    "(Extern",
    "abi=C",
    "abi=Js",
    "ret=I64",
    "ret=F64",
    "ret=I8",
    "ret=Ptr",
    "ret=Unit",
    "(Function",
    "(Function takes_env=true",
    "(Function takes_env=false",
    "(Function async=true",
    "(Function async=false",
    "(Temp",
    "(Block",
    "(ConstInt",
    "(ConstFloat",
    "(ConstBool",
    "(ConstStr",
    "(ConstUnit",
    "(Copy",
    "(Bin",
    "(Neg",
    "(Not",
    "(BitNot",
    "(Call",
    "(CallIndirect",
    "(Await",
    "(CallRuntime",
    "(MakeClosure",
    "(MakeSum",
    "(SumTag",
    "(SumField",
    "(MakeRecord",
    "(RecordField",
    "(SetField",
    "(MakeArray",
    "(ArrayAlloc",
    "(ArrayLen",
    "(ArrayGet",
    "(ArraySet",
    "(Mem",
    "(Goto",
    "(Branch",
    "(Switch",
    "(Return",
    "(Trap",
    "(Diagnostics",
    "(Error",
    "(Primary",
    "(Note",
];

/// The forms of [`KINDS`] only a WASM build's MIR holds, which the native
/// dump never prints: a memory access (`Stmt::Mem`) and a JS extern. The
/// printer takes any module, so it prints them, and its unit test does.
pub const NOT_NATIVE: &[&str] = &["(Mem", "abi=Js"];

/// How far a root got, which its part's first line says (module docs).
pub enum Reached<'a> {
    /// A package module that `check` does not lower: no front end ran.
    NotRun,
    /// An error before MIR, which the HIR dump prints.
    NotChecked,
    /// MIR's result: its diagnostics, or the module, after the async
    /// transform when `async_lowered`, as lowering left it otherwise.
    Lowered {
        result: &'a Result<Module, Vec<Diagnostic>>,
        async_lowered: bool,
    },
}

/// A program's part (module docs): its first line and, once lowered, the
/// module; then `(Diagnostics)`, MIR's alone, each place under its file's
/// label in `labels`.
pub fn dump_program(labels: &Labels, reached: Reached<'_>) -> String {
    let mut d = Dump::default();
    d.open(0, "Mir");
    let mut diagnostics: &[Diagnostic] = &[];
    match reached {
        Reached::NotRun => d.bool_field("runs", false),
        Reached::NotChecked => {
            d.bool_field("runs", true);
            d.bool_field("checked", false);
        }
        Reached::Lowered {
            result,
            async_lowered,
        } => {
            d.bool_field("runs", true);
            d.bool_field("checked", true);
            d.bool_field("lowered", result.is_ok());
            match result {
                Ok(module) => {
                    d.bool_field("async_lowered", async_lowered);
                    d.module(module);
                }
                Err(errors) => diagnostics = errors,
            }
        }
    }
    d.close();
    d.out.push('\n');
    write_diagnostics(&mut d.out, labels, diagnostics);
    d.out
}

/// The text being written.
#[derive(Default)]
struct Dump {
    out: String,
}

impl Dump {
    // --- Lines and values ---

    /// A line, up to its kind.
    fn open(&mut self, depth: usize, kind: &str) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        for _ in 0..depth {
            self.out.push_str("  ");
        }
        self.out.push('(');
        self.out.push_str(kind);
    }

    fn close(&mut self) {
        self.out.push(')');
    }

    /// ` key=value`.
    fn field(&mut self, key: &str, value: &str) {
        let _ = write!(self.out, " {key}={value}");
    }

    fn bool_field(&mut self, key: &str, value: bool) {
        let _ = write!(self.out, " {key}={value}");
    }

    /// ` ` and a value as it is.
    fn value(&mut self, text: &str) {
        self.out.push(' ');
        self.out.push_str(text);
    }

    /// ` ` and `text` quoted.
    fn string(&mut self, text: &str) {
        self.out.push(' ');
        quote(&mut self.out, text);
    }

    /// ` ` and a list in parentheses, its items separated by one space.
    fn list(&mut self, items: impl IntoIterator<Item = String>) {
        let items: Vec<String> = items.into_iter().collect();
        let _ = write!(self.out, " ({})", items.join(" "));
    }

    // --- The module ---

    /// The counts, the externs and every function, after the first line's
    /// fields.
    fn module(&mut self, module: &Module) {
        self.field("functions", &module.functions.len().to_string());
        self.field("externs", &module.externs.len().to_string());
        for e in &module.externs {
            self.open(1, "Extern");
            self.string(&e.symbol);
            let abi = match e.abi {
                ExternAbi::C => "C",
                ExternAbi::Js => "Js",
            };
            self.field("abi", abi);
            self.list(e.params.iter().map(|&c| class(c).to_string()));
            self.field("ret", class(e.ret));
            self.close();
        }
        for f in &module.functions {
            self.function(f);
        }
    }

    /// A function: its line, its temps, one a line, and its blocks.
    fn function(&mut self, f: &Function) {
        self.open(1, "Function");
        self.string(&f.name);
        self.field("params", &f.params.to_string());
        self.bool_field("takes_env", f.takes_env);
        self.field("captures", &f.capture_count.to_string());
        self.field("ret", class(f.ret));
        self.bool_field("async", f.is_async);
        for (i, &c) in f.temps.iter().enumerate() {
            self.open(2, "Temp");
            let _ = write!(self.out, " t{i} {}", class(c));
            self.close();
        }
        for (i, block) in f.blocks.iter().enumerate() {
            self.open(2, "Block");
            let _ = write!(self.out, " bb{i}");
            for stmt in &block.stmts {
                self.stmt(stmt);
            }
            self.terminator(&block.term);
            self.close();
        }
        self.close();
    }

    /// A statement, one line (module docs).
    fn stmt(&mut self, stmt: &Stmt) {
        let (kind, values): (&str, Vec<String>) = match stmt {
            Stmt::ConstInt(dst, v) => ("ConstInt", vec![temp(*dst), v.to_string()]),
            Stmt::ConstFloat(dst, v) => (
                "ConstFloat",
                vec![temp(*dst), format!("{:016x}", v.to_bits())],
            ),
            Stmt::ConstBool(dst, v) => ("ConstBool", vec![temp(*dst), v.to_string()]),
            Stmt::ConstStr(dst, s) => ("ConstStr", vec![temp(*dst), quoted(s)]),
            Stmt::ConstUnit(dst) => ("ConstUnit", vec![temp(*dst)]),
            Stmt::Copy { dst, src } => ("Copy", vec![temp(*dst), temp(*src)]),
            Stmt::Bin {
                dst,
                op,
                class: c,
                lhs,
                rhs,
            } => (
                "Bin",
                vec![
                    temp(*dst),
                    format!("{op:?}"),
                    operand_class(*c).to_string(),
                    temp(*lhs),
                    temp(*rhs),
                ],
            ),
            Stmt::Neg { dst, class: c, src } => (
                "Neg",
                vec![temp(*dst), operand_class(*c).to_string(), temp(*src)],
            ),
            Stmt::Not { dst, src } => ("Not", vec![temp(*dst), temp(*src)]),
            Stmt::BitNot { dst, src } => ("BitNot", vec![temp(*dst), temp(*src)]),
            Stmt::Call { dst, callee, args } => {
                ("Call", vec![dest(*dst), quoted(callee), temps(args)])
            }
            Stmt::CallIndirect {
                dst,
                callee,
                params,
                ret,
                args,
            } => (
                "CallIndirect",
                vec![
                    dest(*dst),
                    temp(*callee),
                    classes(params),
                    class(*ret).to_string(),
                    temps(args),
                ],
            ),
            Stmt::Await { dst, future } => ("Await", vec![dest(*dst), temp(*future)]),
            Stmt::CallRuntime { dst, func, args } => (
                "CallRuntime",
                vec![dest(*dst), format!("{func:?}"), temps(args)],
            ),
            Stmt::MakeClosure {
                dst,
                code,
                captures,
            } => (
                "MakeClosure",
                vec![temp(*dst), quoted(code), typed(captures)],
            ),
            Stmt::MakeSum { dst, tag, fields } => {
                ("MakeSum", vec![temp(*dst), tag.to_string(), typed(fields)])
            }
            Stmt::SumTag { dst, sum } => ("SumTag", vec![temp(*dst), temp(*sum)]),
            Stmt::SumField {
                dst,
                sum,
                index,
                ty,
            } => (
                "SumField",
                vec![
                    temp(*dst),
                    temp(*sum),
                    index.to_string(),
                    class(*ty).to_string(),
                ],
            ),
            Stmt::MakeRecord { dst, fields } => ("MakeRecord", vec![temp(*dst), typed(fields)]),
            Stmt::RecordField {
                dst,
                record,
                index,
                ty,
            } => (
                "RecordField",
                vec![
                    temp(*dst),
                    temp(*record),
                    index.to_string(),
                    class(*ty).to_string(),
                ],
            ),
            Stmt::SetField {
                record,
                index,
                value,
                ty,
            } => (
                "SetField",
                vec![
                    temp(*record),
                    index.to_string(),
                    temp(*value),
                    class(*ty).to_string(),
                ],
            ),
            Stmt::MakeArray { dst, elems } => ("MakeArray", vec![temp(*dst), typed(elems)]),
            Stmt::ArrayAlloc { dst, len } => ("ArrayAlloc", vec![temp(*dst), temp(*len)]),
            Stmt::ArrayLen { dst, arr } => ("ArrayLen", vec![temp(*dst), temp(*arr)]),
            Stmt::ArrayGet {
                dst,
                arr,
                index,
                ty,
            } => (
                "ArrayGet",
                vec![temp(*dst), temp(*arr), temp(*index), class(*ty).to_string()],
            ),
            Stmt::ArraySet {
                arr,
                index,
                value,
                ty,
            } => (
                "ArraySet",
                vec![
                    temp(*arr),
                    temp(*index),
                    temp(*value),
                    class(*ty).to_string(),
                ],
            ),
            Stmt::Mem { dst, op, args } => (
                "Mem",
                vec![dest(*dst), mem_op(*op).to_string(), temps(args)],
            ),
        };
        self.open(3, kind);
        for v in &values {
            self.value(v);
        }
        self.close();
    }

    /// A block's terminator, one line at its statements' depth.
    fn terminator(&mut self, term: &Terminator) {
        match term {
            Terminator::Goto(to) => {
                self.open(3, "Goto");
                self.value(&block(*to));
            }
            Terminator::Branch { cond, then_, else_ } => {
                self.open(3, "Branch");
                self.value(&temp(*cond));
                self.value(&block(*then_));
                self.value(&block(*else_));
            }
            Terminator::Switch {
                disc,
                arms,
                default,
            } => {
                self.open(3, "Switch");
                self.value(&temp(*disc));
                self.list(arms.iter().map(|(v, b)| format!("{v}:{}", block(*b))));
                self.value(&block(*default));
            }
            Terminator::Return(value) => {
                self.open(3, "Return");
                self.value(&dest(*value));
            }
            Terminator::Trap => self.open(3, "Trap"),
        }
        self.close();
    }
}

/// A temp, `t<n>`.
fn temp(t: Temp) -> String {
    format!("t{}", t.0)
}

/// A block, `bb<n>`.
fn block(b: BlockId) -> String {
    format!("bb{}", b.0)
}

/// An optional temp: the temp, or `_`.
fn dest(t: Option<Temp>) -> String {
    t.map_or_else(|| "_".to_string(), temp)
}

/// Temps, a list.
fn temps(ts: &[Temp]) -> String {
    let items: Vec<String> = ts.iter().map(|&t| temp(t)).collect();
    format!("({})", items.join(" "))
}

/// Classes, a list.
fn classes(cs: &[MirTy]) -> String {
    let items: Vec<&str> = cs.iter().map(|&c| class(c)).collect();
    format!("({})", items.join(" "))
}

/// Temps with their classes, a list of `t<n>:<class>`.
fn typed(fields: &[(Temp, MirTy)]) -> String {
    let items: Vec<String> = fields
        .iter()
        .map(|&(t, c)| format!("{}:{}", temp(t), class(c)))
        .collect();
    format!("({})", items.join(" "))
}

/// `text` quoted.
fn quoted(text: &str) -> String {
    let mut out = String::new();
    quote(&mut out, text);
    out
}

/// A class by its variant's name.
fn class(c: MirTy) -> &'static str {
    match c {
        MirTy::I64 => "I64",
        MirTy::F64 => "F64",
        MirTy::I8 => "I8",
        MirTy::Ptr => "Ptr",
        MirTy::Unit => "Unit",
    }
}

/// An operand class by its variant's name.
fn operand_class(c: OperandClass) -> &'static str {
    match c {
        OperandClass::Int => "Int",
        OperandClass::Float => "Float",
        OperandClass::Bool => "Bool",
    }
}

/// A memory operation by its variant's name.
fn mem_op(op: MemOp) -> &'static str {
    match op {
        MemOp::Load8 => "Load8",
        MemOp::Store8 => "Store8",
        MemOp::Load64 => "Load64",
        MemOp::Store64 => "Store64",
        MemOp::Copy => "Copy",
        MemOp::Fill => "Fill",
        MemOp::Pages => "Pages",
        MemOp::Grow => "Grow",
        MemOp::HeapBase => "HeapBase",
        MemOp::Poll => "Poll",
        MemOp::Abort => "Abort",
        MemOp::ClosureWord => "ClosureWord",
        MemOp::ClosureCall => "ClosureCall",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, ExternDecl, RtFunc};
    use nova_diagnostics::{FileId, Span};
    use nova_hir::BinOp;

    fn t(n: u32) -> Temp {
        Temp(n)
    }

    fn bb(n: u32) -> BlockId {
        BlockId(n)
    }

    /// A function of no statement that returns its one temp.
    fn leaf(name: &str, ret: MirTy) -> Function {
        Function {
            name: name.to_string(),
            params: 1,
            takes_env: false,
            capture_count: 0,
            temps: vec![ret],
            ret,
            is_async: false,
            blocks: vec![Block {
                stmts: Vec::new(),
                term: Terminator::Return(Some(t(0))),
            }],
        }
    }

    /// A module holding every statement, terminator, class and extern ABI
    /// once, built by hand: a native build never holds a `Mem` or a JS
    /// extern ([`NOT_NATIVE`]).
    fn every_form() -> Module {
        use MirTy::*;
        let stmts = vec![
            Stmt::ConstInt(t(0), -7),
            Stmt::ConstFloat(t(1), 1.5),
            Stmt::ConstBool(t(2), true),
            Stmt::ConstStr(t(3), "a \"b\"\n".to_string()),
            Stmt::ConstUnit(t(4)),
            Stmt::Copy {
                dst: t(0),
                src: t(5),
            },
            Stmt::Bin {
                dst: t(0),
                op: BinOp::Shl,
                class: OperandClass::Int,
                lhs: t(0),
                rhs: t(5),
            },
            Stmt::Neg {
                dst: t(1),
                class: OperandClass::Float,
                src: t(1),
            },
            Stmt::Not {
                dst: t(2),
                src: t(2),
            },
            Stmt::BitNot {
                dst: t(0),
                src: t(0),
            },
            Stmt::Call {
                dst: Some(t(3)),
                callee: "f.1$i".to_string(),
                args: vec![t(0), t(1)],
            },
            Stmt::Call {
                dst: None,
                callee: "g.2".to_string(),
                args: Vec::new(),
            },
            Stmt::CallIndirect {
                dst: Some(t(0)),
                callee: t(3),
                params: vec![I64, F64],
                ret: I64,
                args: vec![t(0), t(1)],
            },
            Stmt::Await {
                dst: None,
                future: t(3),
            },
            Stmt::CallRuntime {
                dst: Some(t(3)),
                func: RtFunc::IntToStr,
                args: vec![t(0)],
            },
            Stmt::MakeClosure {
                dst: t(3),
                code: "closure$9.9".to_string(),
                captures: vec![(t(0), I64), (t(3), Ptr)],
            },
            Stmt::MakeSum {
                dst: t(3),
                tag: 1,
                fields: vec![(t(0), I64)],
            },
            Stmt::SumTag {
                dst: t(0),
                sum: t(3),
            },
            Stmt::SumField {
                dst: t(0),
                sum: t(3),
                index: 0,
                ty: I64,
            },
            Stmt::MakeRecord {
                dst: t(3),
                fields: vec![(t(1), F64)],
            },
            Stmt::RecordField {
                dst: t(1),
                record: t(3),
                index: 0,
                ty: F64,
            },
            Stmt::SetField {
                record: t(3),
                index: 0,
                value: t(1),
                ty: F64,
            },
            Stmt::MakeArray {
                dst: t(3),
                elems: Vec::new(),
            },
            Stmt::ArrayAlloc {
                dst: t(3),
                len: t(0),
            },
            Stmt::ArrayLen {
                dst: t(0),
                arr: t(3),
            },
            Stmt::ArrayGet {
                dst: t(2),
                arr: t(3),
                index: t(0),
                ty: I8,
            },
            Stmt::ArraySet {
                arr: t(3),
                index: t(0),
                value: t(2),
                ty: I8,
            },
            Stmt::Mem {
                dst: Some(t(0)),
                op: MemOp::Load8,
                args: vec![t(0)],
            },
        ];
        let main = Function {
            name: "main".to_string(),
            params: 0,
            takes_env: false,
            capture_count: 0,
            temps: vec![I64, F64, I8, Ptr, Unit, I64],
            ret: Unit,
            is_async: false,
            blocks: vec![
                Block {
                    stmts,
                    term: Terminator::Branch {
                        cond: t(2),
                        then_: bb(1),
                        else_: bb(2),
                    },
                },
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Switch {
                        disc: t(0),
                        arms: vec![(0, bb(2)), (-1, bb(3))],
                        default: bb(4),
                    },
                },
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Goto(bb(5)),
                },
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Return(None),
                },
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Trap,
                },
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Return(None),
                },
            ],
        };
        let closure = Function {
            takes_env: true,
            capture_count: 2,
            is_async: true,
            ..leaf("closure$9.9", I64)
        };
        Module {
            functions: vec![main, closure, leaf("p.3", Ptr)],
            externs: vec![
                ExternDecl {
                    symbol: "sqrt".to_string(),
                    abi: ExternAbi::C,
                    params: vec![F64],
                    ret: F64,
                },
                ExternDecl {
                    symbol: "js_flag".to_string(),
                    abi: ExternAbi::Js,
                    params: Vec::new(),
                    ret: I8,
                },
            ],
            runtime_alloc: None,
            runtime_exports: Vec::new(),
        }
    }

    const EVERY_FORM: &str = r#"(Mir runs=true checked=true lowered=true async_lowered=false functions=3 externs=2
  (Extern "sqrt" abi=C (F64) ret=F64)
  (Extern "js_flag" abi=Js () ret=I8)
  (Function "main" params=0 takes_env=false captures=0 ret=Unit async=false
    (Temp t0 I64)
    (Temp t1 F64)
    (Temp t2 I8)
    (Temp t3 Ptr)
    (Temp t4 Unit)
    (Temp t5 I64)
    (Block bb0
      (ConstInt t0 -7)
      (ConstFloat t1 3ff8000000000000)
      (ConstBool t2 true)
      (ConstStr t3 "a \x22b\x22\x0a")
      (ConstUnit t4)
      (Copy t0 t5)
      (Bin t0 Shl Int t0 t5)
      (Neg t1 Float t1)
      (Not t2 t2)
      (BitNot t0 t0)
      (Call t3 "f.1$i" (t0 t1))
      (Call _ "g.2" ())
      (CallIndirect t0 t3 (I64 F64) I64 (t0 t1))
      (Await _ t3)
      (CallRuntime t3 IntToStr (t0))
      (MakeClosure t3 "closure$9.9" (t0:I64 t3:Ptr))
      (MakeSum t3 1 (t0:I64))
      (SumTag t0 t3)
      (SumField t0 t3 0 I64)
      (MakeRecord t3 (t1:F64))
      (RecordField t1 t3 0 F64)
      (SetField t3 0 t1 F64)
      (MakeArray t3 ())
      (ArrayAlloc t3 t0)
      (ArrayLen t0 t3)
      (ArrayGet t2 t3 t0 I8)
      (ArraySet t3 t0 t2 I8)
      (Mem t0 Load8 (t0))
      (Branch t2 bb1 bb2))
    (Block bb1
      (Switch t0 (0:bb2 -1:bb3) bb4))
    (Block bb2
      (Goto bb5))
    (Block bb3
      (Return _))
    (Block bb4
      (Trap))
    (Block bb5
      (Return _)))
  (Function "closure$9.9" params=1 takes_env=true captures=2 ret=I64 async=true
    (Temp t0 I64)
    (Block bb0
      (Return t0)))
  (Function "p.3" params=1 takes_env=false captures=0 ret=Ptr async=false
    (Temp t0 Ptr)
    (Block bb0
      (Return t0))))
(Diagnostics)
"#;

    /// One example of every form of [`KINDS`] (spec 5.4 §3.2, ADR 0057):
    /// every statement, terminator, class and extern ABI in a module built
    /// by hand, the transform's flag both ways, MIR's diagnostics with and
    /// without a place and with a note, a program that stopped before MIR
    /// and one that does not run; each form listed, and nothing unlisted.
    #[test]
    fn the_dump_prints_one_example_of_every_form() {
        let labels = [(FileId::DUMMY, "main".to_string())];
        let module = every_form();
        let before = Ok(module);
        let text = dump_program(
            &labels,
            Reached::Lowered {
                result: &before,
                async_lowered: false,
            },
        );
        assert_eq!(text, EVERY_FORM);
        let after = Ok(every_form());
        let text_after = dump_program(
            &labels,
            Reached::Lowered {
                result: &after,
                async_lowered: true,
            },
        );
        assert_eq!(
            text_after,
            EVERY_FORM.replacen("async_lowered=false", "async_lowered=true", 1)
        );
        let errors = Err(vec![
            Diagnostic::error("E0601", "no `main` function found"),
            Diagnostic::error("E0013", "trait bound `(Int, Int): Eq` is not satisfied")
                .with_primary_label(Span::new(4, 9, FileId::DUMMY), "required here")
                .with_note("a tuple has no impls"),
        ]);
        let failed = dump_program(
            &labels,
            Reached::Lowered {
                result: &errors,
                async_lowered: true,
            },
        );
        assert_eq!(
            failed,
            "(Mir runs=true checked=true lowered=false)\n(Diagnostics\n  \
             (Error \"E0601\" \"no `main` function found\")\n  \
             (Error \"E0013\" \"trait bound `(Int, Int): Eq` is not satisfied\"\n    \
             (Primary \"main\":4..9 \"required here\")\n    \
             (Note \"a tuple has no impls\")))\n"
        );
        let unchecked = dump_program(&labels, Reached::NotChecked);
        assert_eq!(unchecked, "(Mir runs=true checked=false)\n(Diagnostics)\n");
        let not_run = dump_program(&labels, Reached::NotRun);
        assert_eq!(not_run, "(Mir runs=false)\n(Diagnostics)\n");

        let mut found = std::collections::HashSet::new();
        for part in [&text, &text_after, &failed, &unchecked, &not_run] {
            for line in part.lines() {
                found.extend(nova_resolver::dump::forms_on(line));
            }
        }
        let missing: Vec<&str> = KINDS
            .iter()
            .copied()
            .filter(|k| !found.contains(*k))
            .collect();
        assert!(missing.is_empty(), "{missing:?} printed nowhere");
        let unlisted: Vec<&String> = found
            .iter()
            .filter(|f| !f.ends_with("=<n>") && !KINDS.contains(&f.as_str()))
            .collect();
        assert!(unlisted.is_empty(), "{unlisted:?} are in no list");
        assert!(NOT_NATIVE.iter().all(|k| KINDS.contains(k)));
    }
}
