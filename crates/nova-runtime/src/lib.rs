//! The Nova runtime: support functions linked into every Nova program.
//!
//! Phase 1 scope (spec `13-RUNTIME.md`, "minimal: panic, allocator, basic
//! types"): string values, console output, and sum-type allocation. All
//! functions use the C ABI so compiled Nova code (Cranelift/LLVM) can call
//! them directly; for `nova run` they are registered as JIT symbols via
//! [`symbols`].
//!
//! **Memory:** Phase 1 allocations are intentionally leaked — the tracing
//! GC (bdwgc, then MMTk) is integrated later in Phase 1/2 (see
//! `docs/adr/0002-phase1-leaking-allocator.md`). Runtime values must not
//! be freed while compiled code may still hold pointers to them.

use std::alloc::{alloc, Layout};

/// A Nova string value: immutable UTF-8, `{ len, ptr }`.
#[repr(C)]
pub struct NovaStr {
    pub len: u64,
    pub ptr: *const u8,
}

/// Leak a Rust string as a `NovaStr` heap value.
fn leak_str(s: String) -> *mut NovaStr {
    let bytes = s.into_bytes().leak();
    Box::into_raw(Box::new(NovaStr {
        len: bytes.len() as u64,
        ptr: bytes.as_ptr(),
    }))
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
    Box::into_raw(Box::new(NovaStr { len, ptr }))
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
    leak_str(s)
}

/// Compare two strings for byte equality.
///
/// # Safety
/// `a` and `b` must be valid `NovaStr` pointers.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_eq(a: *const NovaStr, b: *const NovaStr) -> i8 {
    (as_str(a) == as_str(b)) as i8
}

/// Format an `Int` as a string.
#[no_mangle]
pub extern "C" fn nova_rt_int_to_str(v: i64) -> *mut NovaStr {
    leak_str(v.to_string())
}

/// Format a `Float` as a string.
#[no_mangle]
pub extern "C" fn nova_rt_float_to_str(v: f64) -> *mut NovaStr {
    leak_str(v.to_string())
}

/// Format a `Bool` as `true` / `false`.
#[no_mangle]
pub extern "C" fn nova_rt_bool_to_str(v: i8) -> *mut NovaStr {
    leak_str(if v != 0 { "true" } else { "false" }.to_string())
}

/// Format a `Char` (Unicode scalar value) as a string.
#[no_mangle]
pub extern "C" fn nova_rt_char_to_str(v: i64) -> *mut NovaStr {
    let c = char::from_u32(v as u32).unwrap_or(char::REPLACEMENT_CHARACTER);
    leak_str(c.to_string())
}

/// Allocate `size` bytes (8-aligned) for a heap value — a sum
/// `{ tag, fields... }` or a record `{ fields... }`.
#[no_mangle]
pub extern "C" fn nova_rt_alloc(size: i64) -> *mut u8 {
    let size = (size.max(8)) as usize;
    // SAFETY: size is non-zero and 8-byte alignment is valid.
    let layout = Layout::from_size_align(size, 8).expect("valid heap layout");
    unsafe { alloc(layout) }
}

/// Abort the program with a panic message.
///
/// # Safety
/// `msg` must point to `len` bytes of valid UTF-8.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_panic(msg: *const u8, len: u64) -> ! {
    let m = std::str::from_utf8_unchecked(std::slice::from_raw_parts(msg, len as usize));
    eprintln!("nova: panic: {m}");
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
        ("nova_rt_int_to_str", nova_rt_int_to_str as *const u8),
        ("nova_rt_float_to_str", nova_rt_float_to_str as *const u8),
        ("nova_rt_bool_to_str", nova_rt_bool_to_str as *const u8),
        ("nova_rt_char_to_str", nova_rt_char_to_str as *const u8),
        ("nova_rt_alloc", nova_rt_alloc as *const u8),
        ("nova_rt_panic", nova_rt_panic as *const u8),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn to_string(s: *mut NovaStr) -> String {
        as_str(s).to_string()
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
}
