//! Mid-level IR for the Nova compiler.
//!
//! MIR is a 3-address-style IR over a control-flow graph of basic blocks,
//! produced from typed HIR (`nova-hir`) and consumed by the codegen
//! backends. Lowering to MIR performs:
//!
//! - **Monomorphization**: generic functions are instantiated per unique
//!   concrete type-argument list, reachable from `main` (spec
//!   `14-CODEGEN.md` §6.3). MIR contains no generic types.
//! - **Pattern-match compilation**: `match` lowers to a tag `Switch` over
//!   sum values (or value switches / equality chains for primitives).
//! - **Short-circuit lowering**: `&&` / `||` become control flow.
//!
//! Sum values are boxed: `{ tag: i64, fields: 8 bytes each }`, allocated
//! through the runtime. Strings and function values are opaque pointers.

mod lower;
mod mono;

pub use mono::lower_module;

use nova_hir as hir;

/// A machine-level value class for one MIR temporary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirTy {
    /// 64-bit integer (Int, Char).
    I64,
    /// 64-bit float.
    F64,
    /// Boolean (byte-sized).
    I8,
    /// Opaque pointer (String, sum values, function values).
    Ptr,
    /// No runtime value (unit / diverging results).
    Unit,
}

/// A virtual register within one function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Temp(pub u32);

/// A basic block id within one function; `BlockId(0)` is the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

/// A monomorphized module ready for codegen.
#[derive(Debug, Default)]
pub struct Module {
    pub functions: Vec<Function>,
}

/// A monomorphized function.
#[derive(Debug)]
pub struct Function {
    /// Mangled, unique symbol name (e.g. `identity$i`).
    pub name: String,
    /// The first `params` temps are the parameters.
    pub params: u32,
    pub temps: Vec<MirTy>,
    pub ret: MirTy,
    pub blocks: Vec<Block>,
}

/// A basic block: straight-line statements plus one terminator.
#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub term: Terminator,
}

/// Numeric class of a binary operation's operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandClass {
    Int,
    Float,
    Bool,
}

/// Runtime support functions provided by `nova-runtime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RtFunc {
    /// `(str) -> unit`
    Println,
    /// `(str) -> unit`
    Print,
    /// `(str, str) -> str`
    StrConcat,
    /// `(i64) -> str`
    IntToStr,
    /// `(f64) -> str`
    FloatToStr,
    /// `(i8) -> str`
    BoolToStr,
    /// `(i64 codepoint) -> str`
    CharToStr,
    /// `(str, str) -> i8`
    StrEq,
    /// `(size_bytes) -> ptr` — allocate a heap value (sum or record).
    Alloc,
}

impl RtFunc {
    /// Symbol name registered with the JIT / linker.
    pub fn symbol(self) -> &'static str {
        match self {
            RtFunc::Println => "nova_rt_println",
            RtFunc::Print => "nova_rt_print",
            RtFunc::StrConcat => "nova_rt_str_concat",
            RtFunc::IntToStr => "nova_rt_int_to_str",
            RtFunc::FloatToStr => "nova_rt_float_to_str",
            RtFunc::BoolToStr => "nova_rt_bool_to_str",
            RtFunc::CharToStr => "nova_rt_char_to_str",
            RtFunc::StrEq => "nova_rt_str_eq",
            RtFunc::Alloc => "nova_rt_alloc",
        }
    }

    /// Parameter and return classes: `(params, ret)`.
    pub fn signature(self) -> (Vec<MirTy>, MirTy) {
        match self {
            RtFunc::Println | RtFunc::Print => (vec![MirTy::Ptr], MirTy::Unit),
            RtFunc::StrConcat => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::Ptr),
            RtFunc::IntToStr => (vec![MirTy::I64], MirTy::Ptr),
            RtFunc::FloatToStr => (vec![MirTy::F64], MirTy::Ptr),
            RtFunc::BoolToStr => (vec![MirTy::I8], MirTy::Ptr),
            RtFunc::CharToStr => (vec![MirTy::I64], MirTy::Ptr),
            RtFunc::StrEq => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I8),
            RtFunc::Alloc => (vec![MirTy::I64], MirTy::Ptr),
        }
    }
}

/// A single 3-address statement.
#[derive(Debug)]
pub enum Stmt {
    ConstInt(Temp, i64),
    ConstFloat(Temp, f64),
    ConstBool(Temp, bool),
    /// Interned string literal; codegen emits it as module data.
    ConstStr(Temp, String),
    ConstUnit(Temp),
    Copy {
        dst: Temp,
        src: Temp,
    },
    Bin {
        dst: Temp,
        op: hir::BinOp,
        class: OperandClass,
        lhs: Temp,
        rhs: Temp,
    },
    Neg {
        dst: Temp,
        class: OperandClass,
        src: Temp,
    },
    /// Boolean not.
    Not {
        dst: Temp,
        src: Temp,
    },
    /// Bitwise not (I64).
    BitNot {
        dst: Temp,
        src: Temp,
    },
    /// Direct call to another Nova function by mangled name.
    Call {
        dst: Option<Temp>,
        callee: String,
        args: Vec<Temp>,
    },
    /// Indirect call through a function-pointer temp.
    CallIndirect {
        dst: Option<Temp>,
        callee: Temp,
        params: Vec<MirTy>,
        ret: MirTy,
        args: Vec<Temp>,
    },
    /// Call into the Nova runtime.
    CallRuntime {
        dst: Option<Temp>,
        func: RtFunc,
        args: Vec<Temp>,
    },
    /// Materialize the address of a Nova function as a value.
    FnAddr {
        dst: Temp,
        callee: String,
    },
    /// Allocate and initialize a sum value: `{ tag, fields... }`.
    MakeSum {
        dst: Temp,
        tag: u32,
        fields: Vec<(Temp, MirTy)>,
    },
    /// Load the tag of a sum value.
    SumTag {
        dst: Temp,
        sum: Temp,
    },
    /// Load payload field `index` of a sum value.
    SumField {
        dst: Temp,
        sum: Temp,
        index: u32,
        ty: MirTy,
    },
    /// Allocate and initialize a record value `{ fields... }` (no tag).
    MakeRecord {
        dst: Temp,
        fields: Vec<(Temp, MirTy)>,
    },
    /// Load field `index` of a record value.
    RecordField {
        dst: Temp,
        record: Temp,
        index: u32,
        ty: MirTy,
    },
}

/// Block terminators.
#[derive(Debug)]
pub enum Terminator {
    Goto(BlockId),
    Branch {
        cond: Temp,
        then_: BlockId,
        else_: BlockId,
    },
    /// Multi-way branch on an integer discriminant (sum tag or Int value).
    Switch {
        disc: Temp,
        arms: Vec<(i64, BlockId)>,
        default: BlockId,
    },
    Return(Option<Temp>),
    /// Unreachable (e.g. the default of an exhaustive match).
    Trap,
}

/// Map a concrete HIR type to its machine class.
///
/// Must only be called on monomorphized types (no `Param`/`Var`).
pub fn mir_ty(ty: &hir::Ty) -> MirTy {
    match ty {
        hir::Ty::Int | hir::Ty::Char => MirTy::I64,
        hir::Ty::Float => MirTy::F64,
        hir::Ty::Bool => MirTy::I8,
        hir::Ty::String | hir::Ty::Fn { .. } | hir::Ty::Sum { .. } | hir::Ty::Record { .. } => {
            MirTy::Ptr
        }
        hir::Ty::Unit | hir::Ty::Never => MirTy::Unit,
        // Post-typeck these should not occur; map defensively.
        hir::Ty::Param(_) | hir::Ty::Var(_) | hir::Ty::Error => MirTy::Unit,
    }
}

/// Mangle a function instance name: `name` for non-generic functions,
/// `name$<args>` for monomorphized instances.
pub fn mangle(name: &str, type_args: &[hir::Ty]) -> String {
    if type_args.is_empty() {
        return name.to_string();
    }
    let args: Vec<String> = type_args.iter().map(mangle_ty).collect();
    format!("{name}${}", args.join("_"))
}

fn mangle_ty(ty: &hir::Ty) -> String {
    match ty {
        hir::Ty::Int => "i".to_string(),
        hir::Ty::Float => "f".to_string(),
        hir::Ty::Bool => "b".to_string(),
        hir::Ty::Char => "c".to_string(),
        hir::Ty::String => "s".to_string(),
        hir::Ty::Unit => "u".to_string(),
        hir::Ty::Never => "n".to_string(),
        hir::Ty::Fn { params, ret } => {
            let ps: Vec<String> = params.iter().map(mangle_ty).collect();
            format!("F{}R{}E", ps.join(""), mangle_ty(ret))
        }
        hir::Ty::Sum { def_id, args } => {
            if args.is_empty() {
                format!("S{}", def_id.0)
            } else {
                let a: Vec<String> = args.iter().map(mangle_ty).collect();
                format!("S{}L{}E", def_id.0, a.join(""))
            }
        }
        hir::Ty::Record { def_id, args } => {
            if args.is_empty() {
                format!("R{}", def_id.0)
            } else {
                let a: Vec<String> = args.iter().map(mangle_ty).collect();
                format!("R{}L{}E", def_id.0, a.join(""))
            }
        }
        hir::Ty::Param(_) | hir::Ty::Var(_) | hir::Ty::Error => "X".to_string(),
    }
}
