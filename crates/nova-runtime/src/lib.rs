//! The Nova runtime: support functions linked into every Nova program.
//!
//! Phase 1 scope (spec `13-RUNTIME.md`, "minimal: panic, allocator, basic
//! types"): string values, console output, and sum-type allocation. All
//! functions use the C ABI so compiled Nova code (Cranelift/LLVM) can call
//! them directly; for `nova run` they are registered as JIT symbols via
//! [`symbols`].
//!
//! **Memory:** heap values (records, sums, arrays, closures, strings, and
//! byte buffers) are managed by a conservative mark-and-sweep garbage
//! collector — see [`gc`] and `docs/adr/0002-phase1-leaking-allocator.md`.
//! All heap allocation routes through [`gc::alloc`], which reclaims
//! unreachable objects.

/// Byte-buffer intrinsics for `std/bytes`. `pub`, not private, for the same
/// reason as [`fs`]: its own doc comment explains the boundary it implements
/// (one representation, shared with `NovaStr`, distinguished only by
/// `hir::Ty`). Branch `byte-type`'s own Task 3
/// (`docs/superpowers/plans/2026-08-12-byte-type.md`) added the rest of the
/// byte surface beside this module's first four intrinsics, the same way
/// this crate's other modules build on `task`.
pub mod bytes;
/// The open-file table and its five intrinsics for `std/fs`'s `File`.
/// Private, not `pub` like [`fs`]/[`bytes`]/[`io`]: nothing outside this
/// crate names `file::` directly — the five `nova_rt_file_*` symbols reach
/// the JIT and linked binaries through their `#[no_mangle]` C names and
/// through [`symbols`], both of which need only this module's *items*
/// public, not the module path itself. Branch `file-open-openoptions`'s
/// Task 1 added this module beside `fs`, `io` and `bytes`, reusing `fs`'s
/// per-task slot table rather than owning a second one.
mod file;
/// Filesystem intrinsics for `std/fs`. `pub`, not private: its own doc
/// comment explains the boundary it implements; nothing about that requires
/// hiding the module itself. Branch `std-fs-strings`'s Task 3 and Task 4
/// (`docs/superpowers/plans/2026-08-11-std-fs-strings.md`) built most of this
/// module, and branch `byte-type`'s own Task 4 later added `read`/`write`
/// beside them, the same way this crate's other modules build on `task`.
pub mod fs;
mod gc;
/// The three standard streams' intrinsics for `std/io`. `pub`, not private,
/// for the same reason as [`fs`] and [`bytes`]: its own doc comment explains
/// the boundary it implements. Branch `read-write-stdio`'s Task 1 added this
/// module beside `fs` and `bytes`, reusing `fs`'s per-task slot table rather
/// than owning a second one.
pub mod io;
/// The logger's configuration cell for `std/log`. Private, like [`file`],
/// [`net`], [`poll`] and [`time`]: nothing outside this crate names `log::`
/// directly -- the three `nova_rt_log_*` symbols reach the JIT and linked
/// binaries through their `#[no_mangle]` C names and through [`symbols`].
mod log;
/// The open-socket table and the two-phase, non-blocking `connect` for
/// `std/net`'s `TcpStream`. Private, like [`file`]: nothing outside this
/// crate names `net::` directly -- the `nova_rt_net_*` symbols reach the JIT
/// and linked binaries through their `#[no_mangle]` C names and through
/// [`symbols`], both of which need only this module's *items* public, not
/// the module path itself. Branch `io-poller-std-net`'s Task 3 added this
/// module beside `file`, reusing `file`'s handle-table model rather than
/// inventing a second one, and `poll`'s `RawSocket`/`Interest`/
/// `set_nonblocking` rather than a second socket representation.
mod net;
/// The executor's third wake source (socket readiness), and the two types
/// (`RawSocket`, `Interest`) that let `task.rs` name a socket wait without
/// depending on this module's platform types. Private, like [`gc`] and
/// `file`: nothing outside this crate needs `poll::` directly -- its two
/// callers are both in this crate, `task.rs` (for `wait`) and `net.rs` (for
/// `set_nonblocking` and `wait`).
mod poll;
/// `pub`, not private like [`gc`]: `task`'s ABI constants (`PollFn`,
/// `POLL_READY`, `STATE_SLOT_TAG`, `STATE_SLOT_TEMPS`) are not all read by
/// this crate's own runtime logic -- some exist purely as the documented
/// contract codegen (Task 5's `async_lower.rs`) must reproduce. A private
/// module would make those `pub` items unreachable from outside the crate,
/// which both defeats their purpose and makes rustc's `dead_code` lint treat
/// them as genuinely dead, since nothing inside this crate alone uses them.
pub mod task;
/// The monotonic clock behind `std/time`.
mod time;

/// A Nova string value: immutable UTF-8, `{ len, ptr }`.
#[repr(C)]
pub struct NovaStr {
    pub len: u64,
    pub ptr: *const u8,
}

/// Store a Rust string as a GC-managed `NovaStr` value (its bytes copied into a
/// GC leaf buffer, the header a scanned object that keeps the buffer alive).
///
/// `pub(crate)`, not private: `fs`'s `gc_message` reuses this rather than
/// reproducing `NovaStr { len, ptr }`'s layout a second time, which is
/// precisely the drift class this shared helper exists to avoid.
pub(crate) fn gc_str(s: &str) -> *mut NovaStr {
    let len = s.len();
    // A non-traced byte buffer holding the UTF-8 bytes.
    let buf = gc::alloc(len.max(1), false);
    // SAFETY: `buf` has `len.max(1)` writable bytes.
    unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), buf, len) };
    let node = gc::alloc(std::mem::size_of::<NovaStr>(), true) as *mut NovaStr;
    // SAFETY: `node` points to a fresh `NovaStr`-sized allocation.
    unsafe {
        (*node).len = len as u64;
        (*node).ptr = buf;
    }
    node
}

/// Test-only: allocate a `Bytes` payload, for a test that hands a runtime
/// intrinsic a `Bytes` argument directly rather than going through `std/
/// bytes`'s own Nova-level constructors.
///
/// Delegates to [`bytes::gc_bytes`], which already does exactly this and is
/// `pub(crate)` -- named at the crate root only so `io`'s own tests can reach
/// it as `crate::gc_bytes_for_test` without spelling out the `bytes` module
/// path at each call site.
///
/// `#[cfg(test)]` only, no platform gate: every caller is an ordinary,
/// cross-platform unit test.
#[cfg(test)]
pub(crate) fn gc_bytes_for_test(data: &[u8]) -> *mut NovaStr {
    bytes::gc_bytes(data)
}

/// Read a `NovaStr` back as a `&str`.
///
/// `pub(crate)`, not private: `fs`'s intrinsics read their `NovaStr`
/// arguments through this rather than a second copy of the same unsafe cast.
///
/// # Safety
/// `s` must point to a valid `NovaStr` whose `ptr`/`len` reference valid
/// UTF-8 (guaranteed for strings produced by the compiler and runtime).
pub(crate) unsafe fn as_str<'a>(s: *const NovaStr) -> &'a str {
    let s = &*s;
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(s.ptr, s.len as usize))
}

/// Create a string value from raw bytes (used for string literals).
///
/// # Safety
/// `ptr` must point to `len` bytes of valid UTF-8 that outlive the program
/// (string literal data emitted by codegen).
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_new(ptr: *const u8, len: u64) -> *mut NovaStr {
    // The bytes are static string-literal data (never freed); only the header
    // is GC-managed.
    let node = gc::alloc(std::mem::size_of::<NovaStr>(), true) as *mut NovaStr;
    (*node).len = len;
    (*node).ptr = ptr;
    node
}

/// Print a string followed by a newline to stdout.
///
/// # Safety
/// `s` must be a valid `NovaStr` pointer.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_println(s: *const NovaStr) {
    println!("{}", as_str(s));
}

/// Print a string to stdout without a trailing newline.
///
/// # Safety
/// `s` must be a valid `NovaStr` pointer.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_print(s: *const NovaStr) {
    use std::io::Write;
    let out = std::io::stdout();
    let mut lock = out.lock();
    let _ = lock.write_all(as_str(s).as_bytes());
    let _ = lock.flush();
}

/// Write `s` to stderr with no trailing newline.
///
/// Mirrors [`nova_rt_print`]'s explicit lock-and-flush shape rather than
/// [`nova_rt_println`]'s `println!`, but not its reason. Rust 1.95's
/// `Stdout` is a `LineWriter` unconditionally, so a partial line with no
/// trailing newline can sit in that buffer until something flushes it --
/// there, the explicit flush is load-bearing. `Stderr` carries no such
/// buffer at all (Rust's own source marks its raw handle "not buffered"),
/// so a write reaches the OS on contact regardless of a trailing newline;
/// the flush call below is a harmless no-op kept for symmetry with
/// [`nova_rt_print`], not a mechanism this stream needs.
///
/// # Safety
/// `s` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_eprint(s: *const NovaStr) {
    use std::io::Write;
    let err = std::io::stderr();
    let mut lock = err.lock();
    // SAFETY: forwarding this function's own contract.
    let _ = lock.write_all(unsafe { as_str(s) }.as_bytes());
    let _ = lock.flush();
}

/// Write `s` to stderr followed by a newline.
///
/// # Safety
/// `s` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_eprintln(s: *const NovaStr) {
    // SAFETY: forwarding this function's own contract.
    eprintln!("{}", unsafe { as_str(s) });
}

/// Concatenate two strings into a new string value.
///
/// # Safety
/// `a` and `b` must be valid `NovaStr` pointers.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_concat(a: *const NovaStr, b: *const NovaStr) -> *mut NovaStr {
    let mut s = String::with_capacity((*a).len as usize + (*b).len as usize);
    s.push_str(as_str(a));
    s.push_str(as_str(b));
    gc_str(&s)
}

/// Compare two strings for byte equality.
///
/// # Safety
/// `a` and `b` must be valid `NovaStr` pointers.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_eq(a: *const NovaStr, b: *const NovaStr) -> i8 {
    (as_str(a) == as_str(b)) as i8
}

/// Byte-lexicographic comparison of two strings: `-1`, `0`, or `1`.
///
/// # Safety
/// `a` and `b` must be valid `NovaStr` pointers.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_cmp(a: *const NovaStr, b: *const NovaStr) -> i64 {
    match as_str(a).as_bytes().cmp(as_str(b).as_bytes()) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// FNV-1a hash of a Nova string's bytes, as an `i64`.
///
/// Nova cannot walk a string's bytes itself (`String` has no length, indexing
/// or iteration, and is not FFI-safe), so `std/core`'s `impl Hash for String`
/// reaches this through the `str_hash` builtin. FNV-1a rather than something
/// stronger because it is small, well known, and adequate for a hash map's
/// bucket selection; it is *not* collision-resistant and must not be used for
/// anything security-sensitive.
///
/// The `u64 -> i64` reinterpretation at the end means the result may be
/// negative, so a caller selecting buckets must mask (`hash & (cap - 1)`,
/// which is non-negative for any `i64`) rather than take a remainder.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_hash(s: *const NovaStr) -> i64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in as_str(s).as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h as i64
}

/// Number of Unicode scalar values in `s`.
///
/// Separate from [`nova_rt_str_chars`] so that asking a string's length does
/// not allocate an array of its characters.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_len_chars(s: *const NovaStr) -> i64 {
    as_str(s).chars().count() as i64
}

/// Decompose `s` into a Nova `[Char]`.
///
/// The result must match **exactly** what codegen emits for an array: one
/// block holding `{ len: i64, elem0, elem1, … }`, element `i` at byte offset
/// `8 + 8*i`, allocated *scanned* the way [`nova_rt_alloc`] allocates (it
/// takes no scan parameter and always scans). A `Char` element is its `i64`
/// Unicode scalar value, because `Ty::Char` and `Ty::Int` are both
/// `MirTy::I64`.
///
/// Scanning an array of scalars can retain garbage that happens to look like
/// a pointer. That is the conservative collector's existing behaviour for any
/// `[Int]`, not something new here.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_chars(s: *const NovaStr) -> *mut u8 {
    let chars: Vec<char> = as_str(s).chars().collect();
    let n = chars.len();
    // `8` for the length header plus `8` per element — the same size
    // arithmetic `nova_rt_alloc` is asked for when codegen builds an array.
    // A char count cannot overflow this on a 64-bit target, and `gc::alloc`
    // rejects an undescribable size regardless.
    let block = gc::alloc(8 + 8 * n, true);
    let words = block as *mut i64;
    *words = n as i64;
    for (i, c) in chars.iter().enumerate() {
        *words.add(1 + i) = *c as i64;
    }
    block
}

/// Encode a Nova `[Char]` back into a string.
///
/// A word that is not a valid Unicode scalar value becomes
/// [`char::REPLACEMENT_CHARACTER`] rather than aborting, matching what
/// [`nova_rt_char_to_str`] already does. Nova source cannot produce one —
/// there is no `Int` → `Char` conversion in the language (`let c: Char = 65`
/// is `E0010`, `'a' + 1` is `E0010`, and no such builtin exists) — so this is
/// defensive only.
///
/// # Safety
/// `cs` must point to a Nova array of `Char`: `{ len: i64, elems… }` with
/// element `i` at byte offset `8 + 8*i`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_from_chars(cs: *const u8) -> *mut NovaStr {
    let words = cs as *const i64;
    let n = (*words).max(0) as usize;
    let mut out = String::new();
    for i in 0..n {
        let v = *words.add(1 + i);
        out.push(char::from_u32(v as u32).unwrap_or(char::REPLACEMENT_CHARACTER));
    }
    gc_str(&out)
}

/// Uppercase `s` with full Unicode case mapping.
///
/// Whole-string rather than `Char` → `Char` because the mapping is not 1:1 —
/// `ß` uppercases to `SS` — so a per-character signature could not express it
/// and would silently corrupt such input.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_to_upper(s: *const NovaStr) -> *mut NovaStr {
    gc_str(&as_str(s).to_uppercase())
}

/// Lowercase `s` with full Unicode case mapping. Whole-string for the same
/// reason as [`nova_rt_str_to_upper`].
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_to_lower(s: *const NovaStr) -> *mut NovaStr {
    gc_str(&as_str(s).to_lowercase())
}

/// Format an `Int` as a string.
#[no_mangle]
pub extern "C" fn nova_rt_int_to_str(v: i64) -> *mut NovaStr {
    gc_str(&v.to_string())
}

/// Format a `Float` as a string.
#[no_mangle]
pub extern "C" fn nova_rt_float_to_str(v: f64) -> *mut NovaStr {
    gc_str(&v.to_string())
}

/// Format a `Bool` as `true` / `false`.
#[no_mangle]
pub extern "C" fn nova_rt_bool_to_str(v: i8) -> *mut NovaStr {
    gc_str(if v != 0 { "true" } else { "false" })
}

/// Format a `Char` (Unicode scalar value) as a string.
#[no_mangle]
pub extern "C" fn nova_rt_char_to_str(v: i64) -> *mut NovaStr {
    let c = char::from_u32(v as u32).unwrap_or(char::REPLACEMENT_CHARACTER);
    gc_str(&c.to_string())
}

/// Allocate `size` zeroed bytes for a heap value — a sum `{ tag, fields... }`,
/// record `{ fields... }`, array `{ len, elems... }`, or closure environment.
/// The result is GC-managed and its slots are traced for further pointers.
///
/// Never returns null: a size too large to describe as a heap layout, and one
/// the system allocator cannot satisfy, both abort (see [`gc::alloc`]).
#[no_mangle]
pub extern "C" fn nova_rt_alloc(size: i64) -> *mut u8 {
    gc::alloc(size.max(8) as usize, true)
}

/// Abort if `index` is outside `0..len` (an array bounds violation).
#[no_mangle]
pub extern "C" fn nova_rt_check_bounds(index: i64, len: i64) {
    if index < 0 || index >= len {
        eprintln!("nova: panic: array index {index} out of bounds for length {len}");
        std::process::abort();
    }
}

/// Abort the program with a panic message given as a Nova string.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_panic_str(s: *const NovaStr) -> ! {
    let msg = if s.is_null() { "" } else { as_str(s) };
    eprintln!("nova: panic: {msg}");
    std::process::abort();
}

/// `test_selector() -> i64`. See `Builtin::TestSelector`.
#[no_mangle]
pub extern "C" fn nova_rt_test_selector() -> i64 {
    std::env::var("NOVA_TEST_INDEX")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(-1)
}

/// All runtime symbols, for registration with the JIT (or a linker map).
///
/// **Every `nova_mir::RtFunc` must appear here.** A `RtFunc` variant is
/// declared to both codegen backends by the `rt_funcs!` macro, which guarantees
/// the two backends cannot disagree about the enum — but it says nothing about
/// this table, which is hand-maintained. A variant missing from here compiles
/// clean and only fails when the JIT tries to resolve the symbol, inside
/// `cranelift-jit`'s `finalize_definitions`, which `panic!`s rather than
/// returning an error. `nova-codegen-cranelift`'s
/// `every_rt_func_symbol_is_registered_with_the_jit` pins the containment,
/// since that crate is the one that depends on both this one and `nova-mir`.
pub fn symbols() -> Vec<(&'static str, *const u8)> {
    vec![
        ("nova_rt_str_new", nova_rt_str_new as *const u8),
        ("nova_rt_println", nova_rt_println as *const u8),
        ("nova_rt_print", nova_rt_print as *const u8),
        ("nova_rt_eprint", nova_rt_eprint as *const u8),
        ("nova_rt_eprintln", nova_rt_eprintln as *const u8),
        ("nova_rt_str_concat", nova_rt_str_concat as *const u8),
        ("nova_rt_str_eq", nova_rt_str_eq as *const u8),
        ("nova_rt_str_cmp", nova_rt_str_cmp as *const u8),
        ("nova_rt_str_hash", nova_rt_str_hash as *const u8),
        ("nova_rt_str_len_chars", nova_rt_str_len_chars as *const u8),
        ("nova_rt_str_chars", nova_rt_str_chars as *const u8),
        (
            "nova_rt_str_from_chars",
            nova_rt_str_from_chars as *const u8,
        ),
        ("nova_rt_str_to_upper", nova_rt_str_to_upper as *const u8),
        ("nova_rt_str_to_lower", nova_rt_str_to_lower as *const u8),
        ("nova_rt_int_to_str", nova_rt_int_to_str as *const u8),
        ("nova_rt_float_to_str", nova_rt_float_to_str as *const u8),
        ("nova_rt_bool_to_str", nova_rt_bool_to_str as *const u8),
        ("nova_rt_char_to_str", nova_rt_char_to_str as *const u8),
        ("nova_rt_alloc", nova_rt_alloc as *const u8),
        ("nova_rt_check_bounds", nova_rt_check_bounds as *const u8),
        ("nova_rt_panic_str", nova_rt_panic_str as *const u8),
        ("nova_rt_test_selector", nova_rt_test_selector as *const u8),
        ("nova_rt_task_spawn", task::nova_rt_task_spawn as *const u8),
        (
            "nova_rt_task_block_on",
            task::nova_rt_task_block_on as *const u8,
        ),
        (
            "nova_rt_task_is_done",
            task::nova_rt_task_is_done as *const u8,
        ),
        (
            "nova_rt_task_take_output",
            task::nova_rt_task_take_output as *const u8,
        ),
        (
            "nova_rt_task_release",
            task::nova_rt_task_release as *const u8,
        ),
        (
            "nova_rt_task_yield_future",
            task::nova_rt_task_yield_future as *const u8,
        ),
        (
            "nova_rt_task_sleep_future_nanos",
            task::nova_rt_task_sleep_future_nanos as *const u8,
        ),
        (
            "nova_rt_task_join_future",
            task::nova_rt_task_join_future as *const u8,
        ),
        (
            "nova_rt_task_timeout_future",
            task::nova_rt_task_timeout_future as *const u8,
        ),
        (
            "nova_rt_fs_read_to_string",
            fs::nova_rt_fs_read_to_string as *const u8,
        ),
        (
            "nova_rt_fs_write_string",
            fs::nova_rt_fs_write_string as *const u8,
        ),
        (
            "nova_rt_fs_take_string",
            fs::nova_rt_fs_take_string as *const u8,
        ),
        (
            "nova_rt_fs_last_error_message",
            fs::nova_rt_fs_last_error_message as *const u8,
        ),
        ("nova_rt_fs_temp_dir", fs::nova_rt_fs_temp_dir as *const u8),
        ("nova_rt_fs_exists", fs::nova_rt_fs_exists as *const u8),
        (
            "nova_rt_fs_create_dir",
            fs::nova_rt_fs_create_dir as *const u8,
        ),
        (
            "nova_rt_fs_create_dir_all",
            fs::nova_rt_fs_create_dir_all as *const u8,
        ),
        (
            "nova_rt_fs_remove_file",
            fs::nova_rt_fs_remove_file as *const u8,
        ),
        (
            "nova_rt_fs_remove_dir_all",
            fs::nova_rt_fs_remove_dir_all as *const u8,
        ),
        ("nova_rt_fs_read_dir", fs::nova_rt_fs_read_dir as *const u8),
        (
            "nova_rt_fs_take_string_array",
            fs::nova_rt_fs_take_string_array as *const u8,
        ),
        ("nova_rt_fs_kind", fs::nova_rt_fs_kind as *const u8),
        ("nova_rt_fs_read", fs::nova_rt_fs_read as *const u8),
        (
            "nova_rt_fs_take_bytes",
            fs::nova_rt_fs_take_bytes as *const u8,
        ),
        ("nova_rt_fs_write", fs::nova_rt_fs_write as *const u8),
        (
            "nova_rt_io_stdin_read",
            io::nova_rt_io_stdin_read as *const u8,
        ),
        (
            "nova_rt_io_stdout_write",
            io::nova_rt_io_stdout_write as *const u8,
        ),
        (
            "nova_rt_io_stderr_write",
            io::nova_rt_io_stderr_write as *const u8,
        ),
        (
            "nova_rt_io_stdout_flush",
            io::nova_rt_io_stdout_flush as *const u8,
        ),
        (
            "nova_rt_io_stderr_flush",
            io::nova_rt_io_stderr_flush as *const u8,
        ),
        ("nova_rt_file_open", file::nova_rt_file_open as *const u8),
        ("nova_rt_file_close", file::nova_rt_file_close as *const u8),
        ("nova_rt_file_read", file::nova_rt_file_read as *const u8),
        ("nova_rt_file_write", file::nova_rt_file_write as *const u8),
        ("nova_rt_file_flush", file::nova_rt_file_flush as *const u8),
        (
            "nova_rt_net_connect_future",
            net::nova_rt_net_connect_future as *const u8,
        ),
        ("nova_rt_net_close", net::nova_rt_net_close as *const u8),
        (
            "nova_rt_net_read_future",
            net::nova_rt_net_read_future as *const u8,
        ),
        (
            "nova_rt_net_write_future",
            net::nova_rt_net_write_future as *const u8,
        ),
        (
            "nova_rt_net_read_timeout_future",
            net::nova_rt_net_read_timeout_future as *const u8,
        ),
        ("nova_rt_bytes_len", bytes::nova_rt_bytes_len as *const u8),
        (
            "nova_rt_bytes_from_string",
            bytes::nova_rt_bytes_from_string as *const u8,
        ),
        (
            "nova_rt_bytes_is_utf8",
            bytes::nova_rt_bytes_is_utf8 as *const u8,
        ),
        (
            "nova_rt_bytes_to_string_unchecked",
            bytes::nova_rt_bytes_to_string_unchecked as *const u8,
        ),
        ("nova_rt_bytes_at", bytes::nova_rt_bytes_at as *const u8),
        (
            "nova_rt_bytes_slice",
            bytes::nova_rt_bytes_slice as *const u8,
        ),
        (
            "nova_rt_bytes_concat",
            bytes::nova_rt_bytes_concat as *const u8,
        ),
        (
            "nova_rt_bytes_to_ints",
            bytes::nova_rt_bytes_to_ints as *const u8,
        ),
        (
            "nova_rt_bytes_from_ints",
            bytes::nova_rt_bytes_from_ints as *const u8,
        ),
        ("nova_rt_bytes_eq", bytes::nova_rt_bytes_eq as *const u8),
        (
            "nova_rt_time_now_nanos",
            time::nova_rt_time_now_nanos as *const u8,
        ),
        (
            "nova_rt_time_now_epoch_nanos",
            time::nova_rt_time_now_epoch_nanos as *const u8,
        ),
        (
            "nova_rt_log_config_level",
            log::nova_rt_log_config_level as *const u8,
        ),
        (
            "nova_rt_log_config_to_stderr",
            log::nova_rt_log_config_to_stderr as *const u8,
        ),
        (
            "nova_rt_log_set_config",
            log::nova_rt_log_set_config as *const u8,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn to_string(s: *mut NovaStr) -> String {
        as_str(s).to_string()
    }

    unsafe fn make_str(s: &'static str) -> *mut NovaStr {
        nova_rt_str_new(s.as_ptr(), s.len() as u64)
    }

    #[test]
    fn int_to_str_formats() {
        unsafe {
            assert_eq!(to_string(nova_rt_int_to_str(6765)), "6765");
            assert_eq!(to_string(nova_rt_int_to_str(-1)), "-1");
        }
    }

    #[test]
    fn bool_to_str_formats() {
        unsafe {
            assert_eq!(to_string(nova_rt_bool_to_str(1)), "true");
            assert_eq!(to_string(nova_rt_bool_to_str(0)), "false");
        }
    }

    #[test]
    fn concat_and_eq_work() {
        unsafe {
            let a = nova_rt_str_new("foo".as_ptr(), 3);
            let b = nova_rt_str_new("bar".as_ptr(), 3);
            let ab = nova_rt_str_concat(a, b);
            assert_eq!(to_string(ab), "foobar");
            let c = nova_rt_str_new("foobar".as_ptr(), 6);
            assert_eq!(nova_rt_str_eq(ab, c), 1);
            assert_eq!(nova_rt_str_eq(a, b), 0);
        }
    }

    #[test]
    fn alloc_is_writable() {
        let p = nova_rt_alloc(24);
        assert!(!p.is_null());
        unsafe {
            *(p as *mut i64) = 42;
            assert_eq!(*(p as *const i64), 42);
        }
    }

    #[test]
    fn str_cmp_orders_lexicographically() {
        unsafe {
            let a = make_str("abc");
            let b = make_str("abd");
            let c = make_str("abc");
            assert_eq!(nova_rt_str_cmp(a, b), -1);
            assert_eq!(nova_rt_str_cmp(b, a), 1);
            assert_eq!(nova_rt_str_cmp(a, c), 0);
        }
    }

    #[test]
    fn str_cmp_prefix_is_less() {
        unsafe {
            let a = make_str("ab");
            let b = make_str("abc");
            assert_eq!(nova_rt_str_cmp(a, b), -1);
        }
    }

    #[test]
    fn str_hash_is_deterministic_and_distinguishes() {
        unsafe {
            let a = make_str("hello");
            let b = make_str("hello");
            let c = make_str("world");
            assert_eq!(nova_rt_str_hash(a), nova_rt_str_hash(b));
            assert_ne!(nova_rt_str_hash(a), nova_rt_str_hash(c));
        }
    }

    #[test]
    fn str_hash_handles_empty() {
        unsafe {
            let e = make_str("");
            // Must not panic and must be stable.
            assert_eq!(nova_rt_str_hash(e), nova_rt_str_hash(make_str("")));
        }
    }

    #[test]
    fn str_len_chars_counts_scalars_not_bytes() {
        unsafe {
            assert_eq!(nova_rt_str_len_chars(make_str("café")), 4);
            assert_eq!(nova_rt_str_len_chars(make_str("日本語")), 3);
            assert_eq!(nova_rt_str_len_chars(make_str("")), 0);
            // A 4-byte scalar outside the BMP is still one character.
            assert_eq!(nova_rt_str_len_chars(make_str("🦀")), 1);
        }
    }

    #[test]
    fn str_chars_writes_the_array_layout_codegen_expects() {
        unsafe {
            let block = nova_rt_str_chars(make_str("a→🦀"));
            let words = block as *const i64;
            // Length header first, then one i64 scalar per element.
            assert_eq!(*words, 3);
            assert_eq!(*words.add(1), 'a' as i64);
            assert_eq!(*words.add(2), '→' as i64);
            assert_eq!(*words.add(3), '🦀' as i64);
            // Reading back the written words alone cannot tell a correctly
            // sized 32-byte block (8 header + 8*3 elements) from one that
            // merely has enough allocator slop past a *wrong* declared size
            // for these four words to still land in live memory, and cannot
            // observe the `scan` flag at all — it only affects GC behaviour,
            // not what's readable. Assert what `alloc` actually recorded.
            assert_eq!(gc::object_info(block as usize), Some((32, true)));
            // An empty string still yields a well-formed zero-length array.
            let empty = nova_rt_str_chars(make_str(""));
            assert_eq!(*(empty as *const i64), 0);
            assert_eq!(gc::object_info(empty as usize), Some((8, true)));
        }
    }

    #[test]
    fn str_from_chars_round_trips_and_substitutes_invalid_scalars() {
        unsafe {
            for s in ["", "ascii", "café", "日本語", "🦀🇹🇭"] {
                let back = nova_rt_str_from_chars(nova_rt_str_chars(make_str(s)));
                assert_eq!(nova_rt_str_eq(back, make_str(s)), 1, "round-trip {s}");
            }
            // A surrogate is not a scalar value; substitute, do not abort.
            let block = gc::alloc(16, true) as *mut i64;
            *block = 1;
            *block.add(1) = 0xD800;
            let s = nova_rt_str_from_chars(block as *const u8);
            assert_eq!(as_str(s), "\u{FFFD}");
        }
    }

    #[test]
    fn case_mapping_handles_the_non_one_to_one_cases() {
        unsafe {
            assert_eq!(as_str(nova_rt_str_to_upper(make_str("Straße"))), "STRASSE");
            assert_eq!(as_str(nova_rt_str_to_lower(make_str("HÉLLO"))), "héllo");
            assert_eq!(as_str(nova_rt_str_to_upper(make_str(""))), "");
        }
    }

    /// `case_mapping_handles_the_non_one_to_one_cases` only proves the two
    /// directions are each individually *correct*; it does not by itself
    /// prove that `nova_rt_str_to_upper` and `nova_rt_str_to_lower` are
    /// distinguishable from EACH OTHER or from an identity function that
    /// just returns its input. Both are real mutations a one-character typo
    /// could introduce (swapping which `.to_*case()` call a body makes, or
    /// dropping the case-mapping call entirely), and neither would be caught
    /// by a test that only checks the "forward" direction of each function in
    /// isolation if a caller isn't careful to pick inputs that actually
    /// change under the wrong operation too. Every input below is chosen so
    /// upper/lower/identity all disagree, so any of the three mutations
    /// produces a visibly wrong string here.
    #[test]
    fn case_mapping_is_distinguishable_from_identity_and_from_each_other() {
        unsafe {
            let straße_upper = as_str(nova_rt_str_to_upper(make_str("Straße")));
            assert_eq!(straße_upper, "STRASSE");
            assert_ne!(straße_upper, "Straße"); // rules out identity
            assert_ne!(
                straße_upper,
                as_str(nova_rt_str_to_lower(make_str("Straße")))
            ); // rules out the swap

            let hello_lower = as_str(nova_rt_str_to_lower(make_str("HÉLLO")));
            assert_eq!(hello_lower, "héllo");
            assert_ne!(hello_lower, "HÉLLO"); // rules out identity
            assert_ne!(hello_lower, as_str(nova_rt_str_to_upper(make_str("HÉLLO")))); // rules out the swap

            // A string with no cased characters at all (distinct from a
            // mixed string like "abc123", which still has letters to
            // transform) is unchanged by either direction — the identity
            // outcome is *correct* here, so this specifically exercises that
            // neither wrapper corrupts input it has nothing to do to.
            assert_eq!(
                as_str(nova_rt_str_to_upper(make_str("123 456!"))),
                "123 456!"
            );
            assert_eq!(
                as_str(nova_rt_str_to_lower(make_str("123 456!"))),
                "123 456!"
            );
        }
    }
}
