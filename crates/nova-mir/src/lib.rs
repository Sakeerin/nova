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

mod async_lower;
mod lower;
mod mono;

pub use mono::lower_module;

/// The async ABI this compiler generates against: the state object's slot
/// layout, the future value's slot layout, and the poll statuses.
///
/// Re-exported from the crate root because `nova-runtime` declares the same
/// layout independently — neither crate may depend on the other — and the pin
/// that holds the two copies together has to live in a third crate that
/// depends on both (`nova-codegen-cranelift`).
pub use async_lower::{
    FUTURE_SLOT_POLL, FUTURE_SLOT_STATE, POLL_PENDING, POLL_READY, STATE_MIN_SIZE,
    STATE_SLOT_OUTPUT, STATE_SLOT_TAG, STATE_SLOT_TEMPS,
};

use nova_hir as hir;
use nova_resolver::DefId;

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
    /// `extern` (FFI) symbols the program calls, to be declared as imports.
    pub externs: Vec<ExternDecl>,
}

/// An imported external (C-ABI) function: its raw symbol and ABI classes.
#[derive(Debug, Clone)]
pub struct ExternDecl {
    /// The raw C symbol to import and call (never mangled).
    pub symbol: String,
    pub params: Vec<MirTy>,
    pub ret: MirTy,
}

/// A monomorphized function.
#[derive(Debug)]
pub struct Function {
    /// Mangled, unique symbol name (e.g. `identity.7$i`); the entry point is
    /// the sole exception, emitted under the bare symbol `main`.
    pub name: String,
    /// Number of real value parameters (the first `params` temps after the
    /// optional leading environment pointer).
    pub params: u32,
    /// Whether the ABI has a leading environment pointer (closures and
    /// bare-fn wrappers). When true, temp 0 is the env pointer, temps
    /// `1..=params` are the real parameters, and `capture_count` captured
    /// values are loaded from the environment at entry.
    pub takes_env: bool,
    pub capture_count: u32,
    pub temps: Vec<MirTy>,
    pub ret: MirTy,
    /// Still awaiting the async state-machine transform: this function was
    /// lowered from an `async fn` body, so `ret` is the class of the value the
    /// BODY produces, not of the `Future<T>` its signature declares.
    ///
    /// **No function reaching a codegen backend has this set.**
    /// `lower_module` runs `async_lower::transform` over the finished module,
    /// which rewrites every such function into a poll function plus a wrapper
    /// and clears the flag. A function that still carried it would be emitted
    /// under its original symbol with the wrong return class — invisible
    /// wherever the output type happens to share a register class with a
    /// pointer, a verifier error where it does not.
    pub is_async: bool,
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

/// Defines `RtFunc`'s variants together with [`RtFunc::ALL`] from one
/// variant list, so the two can never disagree.
///
/// Before this, each codegen backend hand-maintained its own copy of "every
/// runtime function" (Cranelift's `ALL_RT`, LLVM's `DECLS`) with no
/// compile-time tie back to this enum. Adding a variant here and forgetting
/// to update those lists compiled clean, and for the LLVM backend silently
/// emitted a call to an undeclared symbol. `ALL` closes that gap: it is
/// generated from the exact identifier list that also declares the enum's
/// variants, in the same macro expansion, so a variant cannot exist without
/// also appearing in `ALL` — there is no second list to forget.
///
/// `symbol()` and `signature()` below stay ordinary hand-written exhaustive
/// `match`es: the compiler already refuses to build if either omits a
/// variant, so generating them buys nothing extra.
macro_rules! rt_funcs {
    ($($(#[$doc:meta])* $variant:ident),+ $(,)?) => {
        /// Runtime support functions provided by `nova-runtime`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum RtFunc {
            $($(#[$doc])* $variant,)+
        }

        impl RtFunc {
            /// Every variant, in the declaration order above — the sole
            /// authoritative list of "all runtime functions". Codegen
            /// backends iterate this instead of hand-maintaining their own
            /// copy (see the `rt_funcs!` docs above).
            pub const ALL: [RtFunc; rt_funcs!(@count $($variant)+)] = [
                $(RtFunc::$variant),+
            ];
        }
    };
    (@count $($v:ident)+) => {
        [$(rt_funcs!(@one $v)),+].len()
    };
    (@one $v:ident) => { () };
}

rt_funcs! {
    /// `(str) -> unit`
    Println,
    /// `(str) -> unit`
    Print,
    /// `(str) -> unit` — write to stderr with no trailing newline.
    EPrint,
    /// `(str) -> unit` — write to stderr followed by a newline.
    EPrintln,
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
    /// `(str, str) -> i64` — lexicographic compare: -1, 0, or 1.
    StrCmp,
    /// `(str) -> i64` — FNV-1a hash of the bytes (may be negative).
    StrHash,
    /// `(str) -> i64` — count of Unicode scalar values.
    StrLenChars,
    /// `(str) -> ptr` — a Nova `[Char]`.
    StrChars,
    /// `(ptr to [Char]) -> str`
    StrFromChars,
    /// `(str) -> str` — full Unicode uppercase.
    StrToUpper,
    /// `(str) -> str` — full Unicode lowercase.
    StrToLower,
    /// `(size_bytes) -> ptr` — allocate a heap value (sum or record).
    Alloc,
    /// `(index, len) -> unit` — abort if `index` is out of `0..len`.
    CheckBounds,
    /// `(str) -> !` — abort with a message.
    Panic,
    /// `() -> i64`. See `Builtin::TestSelector`.
    TestSelector,
    /// `(ptr to { poll_code, state }) -> i64` — queue a task, return its id.
    TaskSpawn,
    /// `(ptr to { poll_code, state }) -> i64` — drive the executor until this
    /// future completes; returns its output slot.
    TaskBlockOn,
    /// `(ptr to { poll_code, state }) -> i8` — whether the task named by that
    /// future's state has completed.
    TaskIsDone,
    /// `(i64 task_id) -> i64` — a completed task's output, taken exactly once.
    /// Declared for completeness of the executor's ABI; no MIR statement emits
    /// it, because a task's output reaches Nova through the future's own state
    /// object instead (`Builtin::TaskOutput`, whose class the runtime's `i64`
    /// return cannot carry for a `Float`).
    TaskTakeOutput,
    /// `(ptr to { poll_code, state }) -> unit` — end the executor's claim on
    /// the task named by that future's state, without handing its output
    /// back. Idempotent.
    TaskRelease,
    /// `() -> ptr to { poll_code, state }` — a fresh future that reports
    /// pending once and then completes. What `std/task`'s `yield_now` awaits.
    TaskYieldFuture,
    /// `(i64 ms) -> ptr to { poll_code, state }` — a fresh future that parks
    /// for `ms` milliseconds via the executor's park set, then completes.
    /// What `std/task`'s `sleep` awaits.
    TaskSleepFuture,
    /// `(ptr to { poll_code, state }) -> ptr to { poll_code, state }` — a
    /// fresh future that parks via the executor's park set until the task
    /// named by the given future's state completes, then completes itself.
    /// What `std/task`'s `join` awaits.
    TaskJoinFuture,
    /// `(str) -> i64` — read the path as UTF-8. `0` on success, with the
    /// contents waiting in `FsTakeString`; otherwise an `IoErrorKind` status
    /// code (`crates/nova-runtime/src/fs.rs`).
    FsReadToString,
    /// `(str, str) -> i64` — write the second string to the path named by the
    /// first, truncating. Status code, as `FsReadToString`.
    FsWriteString,
    /// `() -> str` — take the payload staged by a successful `FsReadToString`.
    FsTakeString,
    /// `() -> str` — take the message staged by a failed filesystem operation.
    FsLastErrorMessage,
    /// `() -> str` — the OS temporary-directory path (`std::env::temp_dir()`).
    FsTempDir,
    /// `(str) -> i8` — whether the path exists.
    FsExists,
    /// `(str) -> i64` — create the directory. Status code, as
    /// `FsReadToString`.
    FsCreateDir,
    /// `(str) -> i64` — create the directory and any missing parents.
    /// Status code, as `FsReadToString`.
    FsCreateDirAll,
    /// `(str) -> i64` — remove the file. Status code, as `FsReadToString`.
    FsRemoveFile,
    /// `(str) -> i64` — remove the directory and everything under it.
    /// Status code, as `FsReadToString`.
    FsRemoveDirAll,
    /// `(str) -> i64` — list the directory's entry names, sorted. Status
    /// code, as `FsReadToString`; on success the names are waiting in
    /// `FsTakeStringArray`.
    FsReadDir,
    /// `() -> ptr` — take the `[String]` staged by a successful `FsReadDir`.
    FsTakeStringArray,
    /// `(str) -> i64` — what the path is: 0 absent, 1 file, 2 directory.
    FsKind,
}

impl RtFunc {
    /// Symbol name registered with the JIT / linker.
    pub fn symbol(self) -> &'static str {
        match self {
            RtFunc::Println => "nova_rt_println",
            RtFunc::Print => "nova_rt_print",
            RtFunc::EPrint => "nova_rt_eprint",
            RtFunc::EPrintln => "nova_rt_eprintln",
            RtFunc::StrConcat => "nova_rt_str_concat",
            RtFunc::IntToStr => "nova_rt_int_to_str",
            RtFunc::FloatToStr => "nova_rt_float_to_str",
            RtFunc::BoolToStr => "nova_rt_bool_to_str",
            RtFunc::CharToStr => "nova_rt_char_to_str",
            RtFunc::StrEq => "nova_rt_str_eq",
            RtFunc::StrCmp => "nova_rt_str_cmp",
            RtFunc::StrHash => "nova_rt_str_hash",
            RtFunc::StrLenChars => "nova_rt_str_len_chars",
            RtFunc::StrChars => "nova_rt_str_chars",
            RtFunc::StrFromChars => "nova_rt_str_from_chars",
            RtFunc::StrToUpper => "nova_rt_str_to_upper",
            RtFunc::StrToLower => "nova_rt_str_to_lower",
            RtFunc::Alloc => "nova_rt_alloc",
            RtFunc::CheckBounds => "nova_rt_check_bounds",
            RtFunc::Panic => "nova_rt_panic_str",
            RtFunc::TestSelector => "nova_rt_test_selector",
            RtFunc::TaskSpawn => "nova_rt_task_spawn",
            RtFunc::TaskBlockOn => "nova_rt_task_block_on",
            RtFunc::TaskIsDone => "nova_rt_task_is_done",
            RtFunc::TaskTakeOutput => "nova_rt_task_take_output",
            RtFunc::TaskRelease => "nova_rt_task_release",
            RtFunc::TaskYieldFuture => "nova_rt_task_yield_future",
            RtFunc::TaskSleepFuture => "nova_rt_task_sleep_future",
            RtFunc::TaskJoinFuture => "nova_rt_task_join_future",
            RtFunc::FsReadToString => "nova_rt_fs_read_to_string",
            RtFunc::FsWriteString => "nova_rt_fs_write_string",
            RtFunc::FsTakeString => "nova_rt_fs_take_string",
            RtFunc::FsLastErrorMessage => "nova_rt_fs_last_error_message",
            RtFunc::FsTempDir => "nova_rt_fs_temp_dir",
            RtFunc::FsExists => "nova_rt_fs_exists",
            RtFunc::FsCreateDir => "nova_rt_fs_create_dir",
            RtFunc::FsCreateDirAll => "nova_rt_fs_create_dir_all",
            RtFunc::FsRemoveFile => "nova_rt_fs_remove_file",
            RtFunc::FsRemoveDirAll => "nova_rt_fs_remove_dir_all",
            RtFunc::FsReadDir => "nova_rt_fs_read_dir",
            RtFunc::FsTakeStringArray => "nova_rt_fs_take_string_array",
            RtFunc::FsKind => "nova_rt_fs_kind",
        }
    }

    /// Parameter and return classes: `(params, ret)`.
    pub fn signature(self) -> (Vec<MirTy>, MirTy) {
        match self {
            RtFunc::Println | RtFunc::Print | RtFunc::EPrint | RtFunc::EPrintln => {
                (vec![MirTy::Ptr], MirTy::Unit)
            }
            RtFunc::StrConcat => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::Ptr),
            RtFunc::IntToStr => (vec![MirTy::I64], MirTy::Ptr),
            RtFunc::FloatToStr => (vec![MirTy::F64], MirTy::Ptr),
            RtFunc::BoolToStr => (vec![MirTy::I8], MirTy::Ptr),
            RtFunc::CharToStr => (vec![MirTy::I64], MirTy::Ptr),
            RtFunc::StrEq => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I8),
            RtFunc::StrCmp => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I64),
            RtFunc::StrHash => (vec![MirTy::Ptr], MirTy::I64),
            RtFunc::StrLenChars => (vec![MirTy::Ptr], MirTy::I64),
            RtFunc::StrChars => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::StrFromChars => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::StrToUpper | RtFunc::StrToLower => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::Alloc => (vec![MirTy::I64], MirTy::Ptr),
            RtFunc::CheckBounds => (vec![MirTy::I64, MirTy::I64], MirTy::Unit),
            RtFunc::Panic => (vec![MirTy::Ptr], MirTy::Unit),
            RtFunc::TestSelector => (vec![], MirTy::I64),
            RtFunc::TaskSpawn | RtFunc::TaskBlockOn => (vec![MirTy::Ptr], MirTy::I64),
            RtFunc::TaskIsDone => (vec![MirTy::Ptr], MirTy::I8),
            RtFunc::TaskTakeOutput => (vec![MirTy::I64], MirTy::I64),
            RtFunc::TaskRelease => (vec![MirTy::Ptr], MirTy::Unit),
            RtFunc::TaskYieldFuture => (vec![], MirTy::Ptr),
            RtFunc::TaskSleepFuture => (vec![MirTy::I64], MirTy::Ptr),
            RtFunc::TaskJoinFuture => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::FsReadToString => (vec![MirTy::Ptr], MirTy::I64),
            RtFunc::FsWriteString => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I64),
            RtFunc::FsTakeString
            | RtFunc::FsLastErrorMessage
            | RtFunc::FsTempDir
            | RtFunc::FsTakeStringArray => (vec![], MirTy::Ptr),
            RtFunc::FsExists => (vec![MirTy::Ptr], MirTy::I8),
            RtFunc::FsCreateDir
            | RtFunc::FsCreateDirAll
            | RtFunc::FsRemoveFile
            | RtFunc::FsRemoveDirAll
            | RtFunc::FsReadDir
            | RtFunc::FsKind => (vec![MirTy::Ptr], MirTy::I64),
        }
    }
}

#[cfg(test)]
mod rt_func_tests {
    use super::RtFunc;

    /// `RtFunc::ALL` is generated by the `rt_funcs!` macro from the same
    /// variant list that defines the enum, so it cannot omit a variant by
    /// construction — but a copy-paste duplicate in that list is still
    /// possible (e.g. writing `Panic` twice instead of adding a 13th
    /// variant). Guard against that directly: every entry's symbol and
    /// signature must be reachable and distinct.
    #[test]
    fn all_lists_every_variant_exactly_once() {
        let symbols: Vec<&str> = RtFunc::ALL.iter().map(|f| f.symbol()).collect();
        let mut sorted = symbols.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            symbols.len(),
            "RtFunc::ALL has a duplicate entry: {symbols:?}"
        );
        // Every variant's signature must resolve without panicking (the
        // match in `signature()` is separately exhaustive, so this always
        // holds — asserted here so a future refactor that breaks that
        // guarantee fails a test, not just a build).
        for f in RtFunc::ALL {
            let _ = f.signature();
        }
    }
}

/// The largest element count `ArrayAlloc` may be handed.
///
/// Both backends compute the allocation size as `8*len + 8` in wrapping `i64`
/// arithmetic, so any longer array would wrap the size — see `ArrayAlloc`. This
/// is the exact largest `len` for which `8*len + 8` still fits in an `i64`.
pub const MAX_ARRAY_LEN: i64 = (i64::MAX - 8) / 8;

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
    /// Indirect call through a function value (fat pointer). `callee` is a
    /// `{ code_ptr, env_ptr }` block; codegen loads both and calls
    /// `code_ptr(env_ptr, args...)`. `params` are the real parameter
    /// classes (excluding the leading env).
    CallIndirect {
        dst: Option<Temp>,
        callee: Temp,
        params: Vec<MirTy>,
        ret: MirTy,
        args: Vec<Temp>,
    },
    /// A suspend point — `future.await` — before the async state-machine
    /// transform has turned it into control flow.
    ///
    /// **No statement reaching a codegen backend is this one.** It is a marker
    /// that `async_lower`'s block split consumes, replacing it with the
    /// sequence that actually performs an await: poll `future` through its own
    /// fat pointer, return `POLL_PENDING` if the answer is pending, and
    /// otherwise copy the awaited value out of the inner state object's output
    /// slot into `dst`'s slot. Every part of that sequence addresses the state
    /// object of the function containing the await, which does not exist until
    /// the transform builds it — so the suspend cannot be lowered into control
    /// flow where it is parsed without inventing that layout twice. Both
    /// backends reject this variant rather than emitting anything for it.
    ///
    /// `future` holds a `{ poll_code, state }` two-word future value, the same
    /// fat-pointer shape `MakeClosure` builds. `dst` is `None` exactly when the
    /// awaited output is unit, matching `Call`'s convention for a call that
    /// produces no value.
    Await {
        dst: Option<Temp>,
        future: Temp,
    },
    /// Call into the Nova runtime.
    CallRuntime {
        dst: Option<Temp>,
        func: RtFunc,
        args: Vec<Temp>,
    },
    /// Build a function value: a 2-word fat pointer `{ code_ptr, env_ptr }`.
    /// `code` is the mangled callee whose address is `code_ptr`; `captures`
    /// are stored into a freshly allocated environment (empty → null env).
    MakeClosure {
        dst: Temp,
        code: String,
        captures: Vec<(Temp, MirTy)>,
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
    /// Store field `index` of a record. Mirrors `RecordField`'s `8 * index`
    /// offset, so reads and writes stay in the same layout.
    SetField {
        record: Temp,
        index: u32,
        value: Temp,
        ty: MirTy,
    },
    /// Allocate and initialize an array `{ len, elems... }`.
    MakeArray {
        dst: Temp,
        elems: Vec<(Temp, MirTy)>,
    },
    /// Allocate an array of `len` elements: `8 + 8*len` zeroed bytes with `len`
    /// stored at offset 0. Elements are left zeroed — the lowering fills them
    /// with its own loop (see `lower_array_repeat`), so neither backend needs a
    /// loop emitter.
    ///
    /// `len` must be in `0..=MAX_ARRAY_LEN`; the lowering guarantees that by
    /// emitting guards that abort first. Both bounds are memory-safety
    /// requirements, because both backends compute the size as `8*len + 8` with
    /// *wrapping* arithmetic:
    ///
    /// - A small negative `len` is harmless on its own — `gc::alloc` clamps
    ///   every request with `size.max(8)`, so `8 + 8*(-1) = 0` still yields a
    ///   block the 8-byte length store fits in — but storing a negative length
    ///   would leave an array whose every access fails, and a large-magnitude
    ///   negative `len` overflows the `8 * len` multiplication into a wild size.
    /// - A `len` above `MAX_ARRAY_LEN` wraps `8*len + 8` back to a *negative*
    ///   size (at `len = 2^60` it is exactly `i64::MIN + 8`), which the same
    ///   `size.max(8)` clamp turns into an 8-byte block. The huge length is then
    ///   stored into that block's header and the fill loop — which carries no
    ///   bounds check by design — writes far past its end.
    ///
    /// Aborting up front reports the mistake where it was made instead.
    ArrayAlloc {
        dst: Temp,
        len: Temp,
    },
    /// Load the length (element count) of an array.
    ArrayLen {
        dst: Temp,
        arr: Temp,
    },
    /// Load `arr[index]` (dynamic index; caller has bounds-checked).
    ArrayGet {
        dst: Temp,
        arr: Temp,
        index: Temp,
        ty: MirTy,
    },
    /// Store `arr[index] = value` (dynamic index; caller has bounds-checked).
    ArraySet {
        arr: Temp,
        index: Temp,
        value: Temp,
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
        hir::Ty::String
        | hir::Ty::Fn { .. }
        | hir::Ty::Sum { .. }
        | hir::Ty::Record { .. }
        | hir::Ty::Array(_)
        | hir::Ty::Future(_) => MirTy::Ptr,
        hir::Ty::Unit | hir::Ty::Never => MirTy::Unit,
        // Post-typeck these should not occur; map defensively. `Assoc` is
        // normalized away by `mono` before lowering, and a projection that
        // survives that is reported as `E0079` rather than reaching here.
        //
        // This arm was *not* merely defensive before that landed: `Assoc`
        // reached it from ordinary source, and mapping to `MirTy::Unit` is
        // not the harmless default it looks like — **unit parameters are
        // dropped from the Cranelift signature**, so a caller passed two
        // arguments to a one-argument function, and a generic function
        // returning a projection returned wrong values with exit 0 and no
        // diagnostic. So reaching here is a compiler bug, but one that must
        // not panic in a library path.
        hir::Ty::Param(_) | hir::Ty::Var(_) | hir::Ty::Error | hir::Ty::Assoc { .. } => MirTy::Unit,
    }
}

/// Mangle a function instance into a unique symbol name.
///
/// The owning `def_id` is folded in so that same-named items from different
/// modules (e.g. a private `helper` in each) get distinct symbols instead of
/// colliding into one at monomorphization; `type_args` further distinguish the
/// instances of a generic function. A non-generic function mangles to
/// `name.<def>` and a monomorphized instance to `name.<def>$<args>` (e.g.
/// `identity.7$i`).
///
/// The program entry point is the sole exception: `lower_module` names it with
/// the bare symbol `main`, which the codegen backends look up by name. It is
/// never passed here for a non-entry function of the same name, because every
/// other function carries its DefId suffix and so can never spell `main`.
pub fn mangle(def_id: DefId, name: &str, type_args: &[hir::Ty]) -> String {
    if type_args.is_empty() {
        return format!("{name}.{}", def_id.0);
    }
    let args: Vec<String> = type_args.iter().map(mangle_ty).collect();
    format!("{name}.{}${}", def_id.0, args.join("_"))
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
        hir::Ty::Array(elem) => format!("A{}E", mangle_ty(elem)),
        // `U` is not otherwise used as a leading letter here (nullary:
        // i/f/b/c/s/u/n; compound: F/S/R/A above), and the mangling must be
        // output-dependent like `Array`'s `A{elem}E` rather than a shared
        // placeholder — see `mangle_ty_distinguishes_futures_by_output_type`.
        hir::Ty::Future(out) => format!("U{}E", mangle_ty(out)),
        // Post-typeck these should not occur; map defensively, matching
        // `mir_ty`'s convention above (`Assoc` is normalized away by `mono`
        // before a type reaches mangling).
        //
        // **This shared `"X"` is a latent symbol collision, not a harmless
        // placeholder.** Four semantically distinct things map to it, and
        // this function's output is `mono`'s `done` dedup key — so two
        // genuinely different instantiations can produce the same symbol,
        // and the second is *skipped as already-emitted* while both dispatch
        // to the first's code. That is the same class as the head-only
        // mangling this project shipped as a miscompile in `d49f896`. It was
        // reproduced by mutation and is pinned; it is unreachable today only
        // because `mono` normalizes `type_args` before requesting a symbol.
        // The right fix is to make them *distinguishable* rather than to
        // panic — `mangle` runs before `E0011`, so aborting would turn a
        // diagnostic into a crash. Queued.
        hir::Ty::Param(_) | hir::Ty::Var(_) | hir::Ty::Error | hir::Ty::Assoc { .. } => {
            "X".to_string()
        }
    }
}
