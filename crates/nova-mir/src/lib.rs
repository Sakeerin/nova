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
    /// Opaque pointer (String, Bytes, sum values, function values).
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
    /// `(f64, i64) -> str` — a `Float` at a fixed number of decimal places.
    FloatFixed,
    /// `(str) -> f64` — decimal text parsed as a `Float`.
    StrToFloat,
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
    /// `(i64 nanos) -> ptr to { poll_code, state }` — a fresh future that
    /// parks for `nanos` nanoseconds via the executor's park set, then
    /// completes. What `std/time`'s `sleep` awaits.
    TaskSleepFutureNanos,
    /// `(ptr to { poll_code, state }) -> ptr to { poll_code, state }` — a
    /// fresh future that parks via the executor's park set until the task
    /// named by the given future's state completes, then completes itself.
    /// What `std/task`'s `join` awaits.
    TaskJoinFuture,
    /// `(i64, ptr) -> ptr to { poll_code, state }` — a future that polls its
    /// inner future under a deadline
    TaskTimeoutFuture,
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
    /// `(str) -> i64` — what the path is: 0 = metadata unavailable (absent,
    /// or unreadable), 1 file, 2 directory.
    FsKind,
    /// `(str) -> i64` — read the path's raw bytes. `0` on success, with the
    /// contents waiting in `FsTakeBytes`; otherwise an `IoErrorKind` status
    /// code, as `FsReadToString`. Unlike `FsReadToString` this cannot produce
    /// `INVALID_DATA`: there is no encoding to violate.
    FsRead,
    /// `() -> bytes` — take the payload staged by a successful `FsRead`.
    FsTakeBytes,
    /// `(str, bytes) -> i64` — write the bytes to the path named by the
    /// first argument, truncating. Status code, as `FsReadToString`.
    FsWrite,
    /// `(ptr, i8, i8, i8, i8, i8, i8) -> i64` — open the path through
    /// `std::fs::OpenOptions`, forwarding the six flags (read, write, append,
    /// truncate, create, create_new) one for one. `0` on success, with the
    /// new fd waiting in `FsTakeBytes`, encoded as an 8-byte little-endian
    /// value; otherwise an `IoErrorKind` status code, as `FsReadToString`.
    FileOpen,
    /// `(i64 fd) -> i64` — close `fd`, dropping the underlying file and
    /// releasing its OS handle. Idempotent: closing an already-closed,
    /// stale, or forged fd still reports success.
    FileClose,
    /// `(i64 fd, i64 max) -> i64` — read up to `max` bytes from `fd`. `0` on
    /// success, with the bytes waiting in `FsTakeBytes`; otherwise an
    /// `IoErrorKind` status code, as `FsReadToString` — including a closed,
    /// stale, or forged fd. An empty payload means end of stream; a short
    /// read does not.
    FileRead,
    /// `(i64 fd, bytes) -> i64` — write the bytes to `fd` with one
    /// `Write::write` call, not a `write_all` loop. Status code, as
    /// `FsReadToString` — including a closed, stale, or forged fd — with the
    /// byte count waiting in `FsTakeBytes` on success, encoded as an 8-byte
    /// little-endian count.
    FileWrite,
    /// `(i64 fd) -> i64` — flush `fd`. Status code, as `FsReadToString`,
    /// including a closed, stale, or forged fd; no payload.
    FileFlush,
    /// `(i64) -> i64` — read up to `max` bytes from stdin. `0` on success,
    /// with the bytes waiting in `FsTakeBytes`; otherwise an `IoErrorKind`
    /// status code, as `FsReadToString`. An empty payload means end of
    /// stream; a short read does not.
    IoStdinRead,
    /// `(bytes) -> i64` — write the bytes to stdout with one `Write::write`
    /// call, not a `write_all` loop. Status code, as `FsReadToString`; on
    /// success the number of bytes actually written is waiting in
    /// `FsTakeBytes`, encoded as an 8-byte little-endian count.
    IoStdoutWrite,
    /// `(bytes) -> i64` — write the bytes to stderr. Mirrors `IoStdoutWrite`
    /// against the other stream.
    IoStderrWrite,
    /// `() -> i64` — flush stdout. Status code, as `FsReadToString`; no
    /// payload.
    IoStdoutFlush,
    /// `() -> i64` — flush stderr. Mirrors `IoStdoutFlush` against the other
    /// stream.
    IoStderrFlush,
    /// `(ptr addr) -> ptr to { poll_code, state }` — a fresh future that
    /// connects to `addr` ("host:port"), non-blockingly.
    ///
    /// **A future constructor, like `TaskSleepFutureNanos`/`TaskJoinFuture` above
    /// — not a status word**, unlike every `FsWrite`/`File*`/`Io*` variant
    /// above it. `.await`ing the returned future produces the `i64` status:
    /// `0` on success, with the new fd waiting in `FsTakeBytes` as an 8-byte
    /// little-endian payload, as `FileOpen`'s own fd; otherwise an
    /// `IoErrorKind` status code, as `FsReadToString`.
    NetConnect,
    /// `(i64 fd) -> i64` — close `fd`, dropping the underlying handle (a
    /// connection or a listener alike) and releasing its OS handle.
    /// Idempotent, as `FileClose`.
    ///
    /// **Not a future constructor** — it returns its status directly, exactly
    /// `FileClose`'s shape. `NetListen` and `NetLocalPort` below are the only
    /// other members of this group that do.
    NetClose,
    /// `(i64 fd, i64 max) -> ptr to { poll_code, state }` — a fresh future
    /// that reads up to `max` bytes from `fd`, non-blockingly.
    ///
    /// A future constructor, as `NetConnect`'s doc comment explains —
    /// `.await`ing it produces `0` on success, with the bytes waiting in
    /// `FsTakeBytes`; otherwise an `IoErrorKind` status code, as
    /// `FsReadToString` — including a closed, stale, or forged fd. An empty
    /// payload means end of stream; a short read does not.
    NetRead,
    /// `(i64 fd, ptr bytes) -> ptr to { poll_code, state }` — a fresh future
    /// that writes the bytes to `fd` with one `Write::write` attempt, not a
    /// `write_all` loop, non-blockingly.
    ///
    /// A future constructor, as `NetConnect`'s doc comment explains —
    /// `.await`ing it produces `0` on success, with the byte count actually
    /// written waiting in `FsTakeBytes`, encoded as an 8-byte little-endian
    /// count, as `FileWrite`; otherwise an `IoErrorKind` status code, as
    /// `FsReadToString` — including a closed, stale, or forged fd.
    NetWrite,
    /// `(i64 fd, i64 max, i64 ms) -> ptr to { poll_code, state }` — a fresh
    /// future that reads up to `max` bytes from `fd`, non-blockingly,
    /// reporting `TIMED_OUT` if `ms` milliseconds pass with nothing to read
    /// first.
    ///
    /// A future constructor, as `NetConnect`'s doc comment explains —
    /// otherwise identical to `NetRead`, including EOF/short-read semantics
    /// and where the bytes land on success.
    NetReadTimeout,
    /// `(ptr addr) -> i64` — bind and listen on `addr` ("host:port"),
    /// registering a non-blocking listening socket.
    ///
    /// **Not a future constructor**, one of the three members of this group
    /// that are not, with `NetClose` and `NetLocalPort`: the `bind` and
    /// `listen` *syscalls* do not block,
    /// so there is no suspension to model. `0` on success, with the new fd
    /// waiting in `FsTakeBytes` as an 8-byte little-endian payload, as
    /// `NetConnect`'s and `FileOpen`'s own; otherwise an `IoErrorKind` status
    /// code, as `FsReadToString`. Closed with `NetClose`, which serves both
    /// kinds of handle.
    ///
    /// A hostname argument still resolves through a blocking lookup, which no
    /// future here parks on — see `nova_rt_net_listen`'s own doc comment in
    /// `crates/nova-runtime/src/net.rs` for that caveat and for how its
    /// resolution differs from `NetConnect`'s.
    NetListen,
    /// `(i64 fd) -> i64` — the port the listening socket behind `fd` is bound
    /// to, which is how a caller learns the kernel's choice after a `NetListen`
    /// on port 0.
    ///
    /// **Not a future constructor**, the third member of this group that is
    /// not, with `NetClose` and `NetListen`: `local_addr` is a `getsockname`
    /// call over bookkeeping the kernel already holds, so there is no
    /// suspension to model. Unlike `NetListen` it takes no address, so it
    /// carries none of that variant's blocking-resolution caveat either.
    ///
    /// `0` on success, **with the port waiting in `FsTakeBytes`** as an 8-byte
    /// little-endian payload rather than in the return value, which is the
    /// status word itself; otherwise an `IoErrorKind` status code, as
    /// `FsReadToString` — including an fd that names a connected stream rather
    /// than a listener, which is a wrong-kind miss and not a port.
    NetLocalPort,
    /// `(i64 fd) -> ptr to { poll_code, state }` — a fresh future that waits
    /// for the next incoming connection on the listening socket behind `fd`.
    ///
    /// A future constructor, as `NetConnect`'s doc comment explains — unlike
    /// `NetClose` and `NetLocalPort`, whose syscalls cannot wait on a peer.
    /// `.await`ing it produces `0` on success, with the accepted connection's
    /// new fd waiting in `FsTakeBytes` as an 8-byte little-endian payload, as
    /// `NetListen`'s and `NetConnect`'s own; otherwise an `IoErrorKind` status
    /// code, as `FsReadToString` — including an fd that names a connected
    /// stream rather than a listener, which is the same wrong-kind miss
    /// `NetLocalPort` describes.
    ///
    /// The accepted fd is an ordinary stream fd, so `NetRead`, `NetWrite`,
    /// `NetReadTimeout` and `NetClose` all act on it unchanged.
    NetAccept,
    /// `(bytes) -> i64` — the byte length. Not a character count: `Bytes` has
    /// no encoding.
    BytesLen,
    /// `(str) -> bytes` — a `Bytes` holding `s`'s UTF-8 bytes.
    BytesFromString,
    /// `(bytes) -> i8` — whether the bytes are valid UTF-8.
    BytesIsUtf8,
    /// `(bytes) -> str` — the bytes as a `String`, without checking UTF-8
    /// validity. The caller must have checked `BytesIsUtf8` first.
    BytesToStringUnchecked,
    /// `(bytes, i64) -> i64` — the byte at the given index, as a value in
    /// `0..=255`. Aborts if the index is out of range.
    BytesAt,
    /// `(bytes, i64, i64) -> bytes` — the bytes in `start..end`, clamped to
    /// the buffer and to `start <= end`.
    BytesSlice,
    /// `(bytes, bytes) -> bytes` — the first buffer's bytes followed by the
    /// second's.
    BytesConcat,
    /// `(bytes) -> ptr` — a Nova `[Int]`, one element per byte.
    BytesToInts,
    /// `(ptr to [Int]) -> bytes` — a `Bytes` holding each array element as one
    /// byte. Aborts if any element is outside `0..=255`.
    BytesFromInts,
    /// `(bytes, bytes) -> i8` — byte-for-byte equality.
    BytesEq,
    /// `() -> i64` — nanoseconds since the runtime's process epoch.
    TimeNowNanos,
    /// `() -> i64` — nanoseconds since the **Unix** epoch. Distinct from
    /// `TimeNowNanos`, which is process-relative.
    TimeNowEpochNanos,
    /// `() -> i64` — the logger's configured threshold, as `LogLevel::to_int`
    /// numbers it.
    LogConfigLevel,
    /// `() -> i64` — `1` for stderr, `0` for stdout.
    LogConfigToStderr,
    /// `(i64 level, i64 to_stderr) -> unit` — install a logger configuration,
    /// overwriting any previous one.
    LogSetConfig,
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
            RtFunc::FloatFixed => "nova_rt_float_fixed",
            RtFunc::StrToFloat => "nova_rt_str_to_float",
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
            RtFunc::TaskSleepFutureNanos => "nova_rt_task_sleep_future_nanos",
            RtFunc::TaskJoinFuture => "nova_rt_task_join_future",
            RtFunc::TaskTimeoutFuture => "nova_rt_task_timeout_future",
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
            RtFunc::FsRead => "nova_rt_fs_read",
            RtFunc::FsTakeBytes => "nova_rt_fs_take_bytes",
            RtFunc::FsWrite => "nova_rt_fs_write",
            RtFunc::FileOpen => "nova_rt_file_open",
            RtFunc::FileClose => "nova_rt_file_close",
            RtFunc::FileRead => "nova_rt_file_read",
            RtFunc::FileWrite => "nova_rt_file_write",
            RtFunc::FileFlush => "nova_rt_file_flush",
            RtFunc::IoStdinRead => "nova_rt_io_stdin_read",
            RtFunc::IoStdoutWrite => "nova_rt_io_stdout_write",
            RtFunc::IoStderrWrite => "nova_rt_io_stderr_write",
            RtFunc::IoStdoutFlush => "nova_rt_io_stdout_flush",
            RtFunc::IoStderrFlush => "nova_rt_io_stderr_flush",
            RtFunc::NetConnect => "nova_rt_net_connect_future",
            RtFunc::NetClose => "nova_rt_net_close",
            RtFunc::NetRead => "nova_rt_net_read_future",
            RtFunc::NetWrite => "nova_rt_net_write_future",
            RtFunc::NetReadTimeout => "nova_rt_net_read_timeout_future",
            RtFunc::NetListen => "nova_rt_net_listen",
            RtFunc::NetLocalPort => "nova_rt_net_local_port",
            RtFunc::NetAccept => "nova_rt_net_accept_future",
            RtFunc::BytesLen => "nova_rt_bytes_len",
            RtFunc::BytesFromString => "nova_rt_bytes_from_string",
            RtFunc::BytesIsUtf8 => "nova_rt_bytes_is_utf8",
            RtFunc::BytesToStringUnchecked => "nova_rt_bytes_to_string_unchecked",
            RtFunc::BytesAt => "nova_rt_bytes_at",
            RtFunc::BytesSlice => "nova_rt_bytes_slice",
            RtFunc::BytesConcat => "nova_rt_bytes_concat",
            RtFunc::BytesToInts => "nova_rt_bytes_to_ints",
            RtFunc::BytesFromInts => "nova_rt_bytes_from_ints",
            RtFunc::BytesEq => "nova_rt_bytes_eq",
            RtFunc::TimeNowNanos => "nova_rt_time_now_nanos",
            RtFunc::TimeNowEpochNanos => "nova_rt_time_now_epoch_nanos",
            RtFunc::LogConfigLevel => "nova_rt_log_config_level",
            RtFunc::LogConfigToStderr => "nova_rt_log_config_to_stderr",
            RtFunc::LogSetConfig => "nova_rt_log_set_config",
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
            RtFunc::FloatFixed => (vec![MirTy::F64, MirTy::I64], MirTy::Ptr),
            RtFunc::StrToFloat => (vec![MirTy::Ptr], MirTy::F64),
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
            RtFunc::TaskSleepFutureNanos => (vec![MirTy::I64], MirTy::Ptr),
            RtFunc::TaskJoinFuture => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::TaskTimeoutFuture => (vec![MirTy::I64, MirTy::Ptr], MirTy::Ptr),
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
            | RtFunc::FsKind
            | RtFunc::FsRead => (vec![MirTy::Ptr], MirTy::I64),
            RtFunc::FsTakeBytes => (vec![], MirTy::Ptr),
            RtFunc::FsWrite => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I64),
            RtFunc::FileOpen => (
                vec![
                    MirTy::Ptr,
                    MirTy::I8,
                    MirTy::I8,
                    MirTy::I8,
                    MirTy::I8,
                    MirTy::I8,
                    MirTy::I8,
                ],
                MirTy::I64,
            ),
            RtFunc::FileClose | RtFunc::FileFlush => (vec![MirTy::I64], MirTy::I64),
            RtFunc::FileRead => (vec![MirTy::I64, MirTy::I64], MirTy::I64),
            RtFunc::FileWrite => (vec![MirTy::I64, MirTy::Ptr], MirTy::I64),
            RtFunc::IoStdinRead => (vec![MirTy::I64], MirTy::I64),
            RtFunc::IoStdoutWrite | RtFunc::IoStderrWrite => (vec![MirTy::Ptr], MirTy::I64),
            RtFunc::IoStdoutFlush | RtFunc::IoStderrFlush => (vec![], MirTy::I64),
            // Most of this group return `MirTy::Ptr` (a future's `{ poll_code,
            // state }` pointer), not `MirTy::I64` — the same shape
            // `TaskSleepFutureNanos`/`TaskJoinFuture` use above, unlike every
            // `FsWrite`/`File*`/`Io*` entry directly above this group.
            // `NetClose`, `NetListen` and `NetLocalPort` are the three that
            // return `MirTy::I64`, exactly `FileClose`'s shape. `NetListen` and
            // `NetConnect` take the same single `Ptr` (an address string) and
            // differ only in the return: `I64` for the status word, `Ptr` for
            // the future.
            //
            // This table *declares* those classes and checks nothing itself.
            // Its consumers are the two backends, which build every
            // declaration and call from it, and LLVM's
            // `every_rt_func_is_declared_with_its_real_signature` pins each
            // entry against the IR that comes out. What nothing pins is a
            // *lowering arm's choice of variant*: `lower.rs` naming
            // `NetConnect` where it meant `NetListen` would emit a well-formed
            // call with the wrong return class, caught by neither this table
            // nor that test.
            //
            // `NetClose` and `NetLocalPort` sharpen that hazard, because they
            // are *indistinguishable here* -- both `(vec![I64], I64)` -- so a
            // lowering arm confusing the two would emit a call this table
            // cannot fault and that IR test cannot fault either, and the only
            // thing that separates them is which symbol `symbol()` names. The
            // typeck-side `builtin_signatures_are_what_the_std_call_sites_use`
            // is equally blind to the swap.
            //
            // Only a test that *runs* the compiled program catches it, and
            // both directions now do. `NetClose` lowered to `NetLocalPort`
            // would run `nova_rt_net_local_port` against a stream fd, miss the
            // listener kind check, and make `TcpStream::close` report `Err`, so
            // `tests/runtime/net_lifetime.stdout`'s first golden line
            // (`close: ok`) and `net_roundtrip.stdout`'s last one both fail.
            // `NetLocalPort` lowered to `NetClose` was unpinned until
            // `tests/runtime/net_listener_accept.nova` -- the first and still
            // only fixture that calls `local_port`. That direction was
            // measured, not reasoned about: with `local_port`'s body pointed at
            // `net_close`, that fixture's first golden line flips to
            // `server: port in range false` (the close returns status `0`, so
            // the `Ok` arm still runs and decodes the payload slot `bind`
            // already emptied -- to something outside 1..=65535, which is what
            // that flipped line measures) *and* the now-closed listener makes
            // the next line `nova: panic: accept: socket is not open`, exit
            // 127. So the swap fails both the golden and the `.success()`
            // assertion, and
            // fails them on the fixture's *first* line rather than hanging --
            // which is the whole reason that line asserts the port's range
            // instead of trusting the client's `connect` to notice.
            //
            // `NetAccept` shares that same single-`I64` parameter list but
            // *not* the hazard, and the reason is worth stating rather than
            // leaving a reader to check: it returns `Ptr`, so confusing it
            // with either of those two in a lowering arm changes the return
            // class and this table faults it, exactly as it would fault
            // `NetConnect` swapped for `NetListen`. The indistinguishable pair
            // stays a pair.
            RtFunc::NetConnect => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::NetClose => (vec![MirTy::I64], MirTy::I64),
            RtFunc::NetRead => (vec![MirTy::I64, MirTy::I64], MirTy::Ptr),
            RtFunc::NetWrite => (vec![MirTy::I64, MirTy::Ptr], MirTy::Ptr),
            RtFunc::NetReadTimeout => (vec![MirTy::I64, MirTy::I64, MirTy::I64], MirTy::Ptr),
            RtFunc::NetListen => (vec![MirTy::Ptr], MirTy::I64),
            RtFunc::NetLocalPort => (vec![MirTy::I64], MirTy::I64),
            RtFunc::NetAccept => (vec![MirTy::I64], MirTy::Ptr),
            RtFunc::BytesLen => (vec![MirTy::Ptr], MirTy::I64),
            RtFunc::BytesFromString => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::BytesIsUtf8 => (vec![MirTy::Ptr], MirTy::I8),
            RtFunc::BytesToStringUnchecked => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::BytesAt => (vec![MirTy::Ptr, MirTy::I64], MirTy::I64),
            RtFunc::BytesSlice => (vec![MirTy::Ptr, MirTy::I64, MirTy::I64], MirTy::Ptr),
            RtFunc::BytesConcat => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::Ptr),
            RtFunc::BytesToInts => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::BytesFromInts => (vec![MirTy::Ptr], MirTy::Ptr),
            RtFunc::BytesEq => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I8),
            RtFunc::TimeNowNanos => (vec![], MirTy::I64),
            RtFunc::TimeNowEpochNanos => (vec![], MirTy::I64),
            RtFunc::LogConfigLevel => (vec![], MirTy::I64),
            RtFunc::LogConfigToStderr => (vec![], MirTy::I64),
            RtFunc::LogSetConfig => (vec![MirTy::I64, MirTy::I64], MirTy::Unit),
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
        | hir::Ty::Bytes
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
        // Distinct from `String`'s `"s"`: two types mangling to the same
        // string is the exact miscompile class `d49f896` shipped (see the
        // defensive `"X"` arm's comment below) -- two monomorphized instances
        // would collide on one symbol and both dispatch to the first one's
        // code.
        hir::Ty::Bytes => "y".to_string(),
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
        // i/f/b/c/s/y/u/n; compound: F/S/R/A above), and the mangling must be
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
