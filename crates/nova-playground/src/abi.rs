//! The C ABI the playground's page calls, on wasm32 alone (spec
//! `docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md`
//! §3.1): [`alloc`] and [`dealloc`], a buffer for the source's UTF-8;
//! [`compile`], which returns nine words; and [`panic_info`], two words a
//! panic hook fills. No `wasm-bindgen`, and the module imports nothing.
//!
//! A panic traps (`panic = "abort"` on wasm32) after the hook has stored
//! its message, so the page takes [`panic_info`]'s address when it
//! instantiates the module and reads the message from memory after a
//! trap, never calling the dead instance again.

// All of the crate's `unsafe` is here, against its `#![deny(unsafe_code)]`:
// raw pointers from JS, and `#[no_mangle]`, which the lint counts (6.6
// inventories `unsafe` by file).
#![allow(unsafe_code)]

use std::cell::RefCell;
use std::sync::Once;

use crate::Outcome;

/// The last compile's buffers, and the nine words naming them, kept until
/// the next compile.
#[derive(Default)]
struct Last {
    wasm: Vec<u8>,
    js: String,
    text: String,
    words: [u32; 9],
}

/// The last panic's message, and the two words naming it.
#[derive(Default)]
struct Panic {
    message: String,
    words: [u32; 2],
}

thread_local! {
    static LAST: RefCell<Last> = RefCell::new(Last::default());
    static PANIC: RefCell<Panic> = RefCell::new(Panic::default());
}

/// An address in the module's memory, as a word: wasm32's pointers are 32
/// bits.
fn word<T>(ptr: *const T) -> u32 {
    ptr as usize as u32
}

/// A length, as a word.
fn length(len: usize) -> u32 {
    len as u32
}

/// A buffer of `len` bytes, for the page to fill with the source's UTF-8
/// and pass to [`compile`], then free with [`dealloc`].
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Free a buffer [`alloc`] gave.
///
/// # Safety
///
/// `ptr` and `len` are a buffer [`alloc`] returned, not yet freed.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    // SAFETY: `alloc` made this buffer as a `Vec<u8>` of this capacity.
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len.max(1)) });
}

/// Compile the `len` bytes of UTF-8 at `ptr` with [`crate::compile`], and
/// return the address of nine words: the status (0 compiled, the text
/// holding its warnings; 1 failed; 2 an internal error); `app.wasm`'s
/// address and length; `app.js`'s; the text's; the error count; and
/// `page`, 0 or 1. What they name lives until the next call.
///
/// # Safety
///
/// `ptr` and `len` name `len` bytes the caller may read, such as a buffer
/// [`alloc`] returned.
#[no_mangle]
pub unsafe extern "C" fn compile(ptr: *const u8, len: usize) -> *const u32 {
    set_panic_hook();
    // The last compile's buffers go first, so two never coexist.
    LAST.with(|last| *last.borrow_mut() = Last::default());
    // SAFETY: the caller's promise.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let outcome = crate::compile(&String::from_utf8_lossy(bytes));
    let (status, wasm, js, text, errors, page) = match outcome {
        Outcome::Compiled {
            wasm,
            js,
            page,
            warnings,
        } => (0, wasm, js, warnings, 0, page),
        Outcome::Failed { text, errors } => (1, Vec::new(), String::new(), text, errors, false),
        Outcome::Internal { text } => (2, Vec::new(), String::new(), text, 0, false),
    };
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        last.words = [
            status,
            word(wasm.as_ptr()),
            length(wasm.len()),
            word(js.as_ptr()),
            length(js.len()),
            word(text.as_ptr()),
            length(text.len()),
            length(errors),
            u32::from(page),
        ];
        // Moving a buffer moves its handle, not its bytes.
        last.wasm = wasm;
        last.js = js;
        last.text = text;
        last.words.as_ptr()
    })
}

/// The address of two words, the last panic's message's address and
/// length, which the panic hook fills before the trap: zero until a panic.
#[no_mangle]
pub extern "C" fn panic_info() -> *const u32 {
    PANIC.with(|panic| panic.borrow().words.as_ptr())
}

/// Keep each panic's message where [`panic_info`] says, from the first
/// compile on.
fn set_panic_hook() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let message = info.to_string();
            let _ = PANIC.try_with(|panic| {
                if let Ok(mut panic) = panic.try_borrow_mut() {
                    panic.words = [word(message.as_ptr()), length(message.len())];
                    panic.message = message;
                }
            });
        }));
    });
}
