//! The state of `std/ui`: one record per thread, which `std/ui` makes the
//! first time it needs it (spec
//! `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
//! §4). The current computation and owner, the effects waiting to run, and
//! the roots a program keeps all hang from it, so it is a collector root
//! from its first store for the rest of the thread's life.

use std::cell::Cell;

use crate::gc;

thread_local! {
    /// The record's address, or 0 before the first store.
    static STATE: Cell<usize> = const { Cell::new(0) };
}

/// `ui_state_present() -> Bool`: whether a state is stored, as `1` or `0`.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_ui_state_present() -> i8 {
    i8::from(STATE.with(Cell::get) != 0)
}

/// `ui_state_load<T>() -> T`: the record stored, or null before the first
/// store, which `std/ui` never asks for.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_ui_state_load() -> *mut u8 {
    STATE.with(Cell::get) as *mut u8
}

/// `ui_state_store<T>(state: T)`: keep `state`, rooting it and releasing the
/// root of the record it replaces.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_ui_state_store(state: *mut u8) {
    gc::add_root(state);
    let old = STATE.with(|s| s.replace(state as usize));
    if old != 0 {
        gc::remove_root(old as *mut u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stored record is a root until another replaces it, and the state
    /// reads back what was stored.
    #[test]
    fn a_stored_state_reads_back_and_is_rooted() {
        assert_eq!(nova_rt_ui_state_present(), 0);
        let a = crate::nova_rt_alloc(16);
        nova_rt_ui_state_store(a);
        assert_eq!(nova_rt_ui_state_present(), 1);
        assert_eq!(nova_rt_ui_state_load(), a);
        assert_eq!(gc::root_count(a as usize), 1);
        let b = crate::nova_rt_alloc(16);
        nova_rt_ui_state_store(b);
        assert_eq!(nova_rt_ui_state_load(), b);
        assert_eq!(gc::root_count(b as usize), 1);
        assert_eq!(gc::root_count(a as usize), 0);
    }
}
