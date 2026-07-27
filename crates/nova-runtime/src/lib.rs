//! The Nova runtime: support functions linked into every Nova program.
//!
//! Phase 1 scope (spec `13-RUNTIME.md`, "minimal: panic, allocator, basic
//! types"): string values, console output, and sum-type allocation. All
//! functions use the C ABI so compiled Nova code (Cranelift/LLVM) can call
//! them directly; for `nova run` they are registered as JIT symbols via
//! [`symbols`].
//!
//! **Memory:** heap values (records, sums, arrays, closures, and strings) are
//! managed by a conservative mark-and-sweep garbage collector — see
//! [`gc`] and `docs/adr/0002-phase1-leaking-allocator.md`. All heap allocation
//! routes through [`gc::alloc`], which reclaims unreachable objects.

mod gc;

/// A Nova string value: immutable UTF-8, `{ len, ptr }`.
#[repr(C)]
pub struct NovaStr {
    pub len: u64,
    pub ptr: *const u8,
}

/// Store a Rust string as a GC-managed `NovaStr` value (its bytes copied into a
/// GC leaf buffer, the header a scanned object that keeps the buffer alive).
fn gc_str(s: &str) -> *mut NovaStr {
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

/// Read a `NovaStr` back as a `&str`.
///
/// # Safety
/// `s` must point to a valid `NovaStr` whose `ptr`/`len` reference valid
/// UTF-8 (guaranteed for strings produced by the compiler and runtime).
unsafe fn as_str<'a>(s: *const NovaStr) -> &'a str {
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

/// All runtime symbols, for registration with the JIT (or a linker map).
pub fn symbols() -> Vec<(&'static str, *const u8)> {
    vec![
        ("nova_rt_str_new", nova_rt_str_new as *const u8),
        ("nova_rt_println", nova_rt_println as *const u8),
        ("nova_rt_print", nova_rt_print as *const u8),
        ("nova_rt_str_concat", nova_rt_str_concat as *const u8),
        ("nova_rt_str_eq", nova_rt_str_eq as *const u8),
        ("nova_rt_str_cmp", nova_rt_str_cmp as *const u8),
        ("nova_rt_str_hash", nova_rt_str_hash as *const u8),
        ("nova_rt_str_len_chars", nova_rt_str_len_chars as *const u8),
        ("nova_rt_str_chars", nova_rt_str_chars as *const u8),
        ("nova_rt_int_to_str", nova_rt_int_to_str as *const u8),
        ("nova_rt_float_to_str", nova_rt_float_to_str as *const u8),
        ("nova_rt_bool_to_str", nova_rt_bool_to_str as *const u8),
        ("nova_rt_char_to_str", nova_rt_char_to_str as *const u8),
        ("nova_rt_alloc", nova_rt_alloc as *const u8),
        ("nova_rt_check_bounds", nova_rt_check_bounds as *const u8),
        ("nova_rt_panic_str", nova_rt_panic_str as *const u8),
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
}
