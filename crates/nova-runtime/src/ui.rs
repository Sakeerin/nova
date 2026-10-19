//! The state of `std/ui`: one record per thread, which `std/ui` makes the
//! first time it needs it (spec
//! `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
//! §4). The current computation and owner, the effects waiting to run, and
//! the roots a program keeps all hang from it, so it is a collector root
//! from its first store for the rest of the thread's life.
//!
//! `std/ui/router` keeps a record of its own the same way, a second root
//! (spec `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
//! §3.5): the router's location, its mode, and the parameters of the route
//! being rendered.

use std::cell::Cell;
use std::thread::LocalKey;

use crate::gc;

thread_local! {
    /// `std/ui`'s record's address, or 0 before the first store.
    static STATE: Cell<usize> = const { Cell::new(0) };
    /// `std/ui/router`'s record's address, or 0 before the first store.
    static ROUTER_STATE: Cell<usize> = const { Cell::new(0) };
}

/// Keep `state` in `slot`, rooting it and releasing the root of the record
/// it replaces.
fn store(slot: &'static LocalKey<Cell<usize>>, state: *mut u8) {
    gc::add_root(state);
    let old = slot.with(|s| s.replace(state as usize));
    if old != 0 {
        gc::remove_root(old as *mut u8);
    }
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
    store(&STATE, state);
}

/// `router_state_present() -> Bool`: whether the router's state is stored,
/// as `1` or `0`.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_router_state_present() -> i8 {
    i8::from(ROUTER_STATE.with(Cell::get) != 0)
}

/// `router_state_load<T>() -> T`: the router's record, or null before the
/// first store, which `std/ui/router` never asks for.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_router_state_load() -> *mut u8 {
    ROUTER_STATE.with(Cell::get) as *mut u8
}

/// `router_state_store<T>(state: T)`: keep the router's `state`, rooting it
/// and releasing the root of the record it replaces.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_router_state_store(state: *mut u8) {
    store(&ROUTER_STATE, state);
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

    /// The router's slot is a second root, apart from `std/ui`'s: storing
    /// in it leaves `std/ui`'s slot empty, it reads back what was stored,
    /// and a record replaced in it is no longer a root.
    #[test]
    fn the_routers_state_is_a_root_of_its_own() {
        assert_eq!(nova_rt_router_state_present(), 0);
        let a = crate::nova_rt_alloc(16);
        nova_rt_router_state_store(a);
        assert_eq!(nova_rt_router_state_present(), 1);
        assert_eq!(nova_rt_ui_state_present(), 0);
        assert_eq!(nova_rt_router_state_load(), a);
        assert_eq!(gc::root_count(a as usize), 1);
        let b = crate::nova_rt_alloc(16);
        nova_rt_router_state_store(b);
        assert_eq!(nova_rt_router_state_load(), b);
        assert_eq!(gc::root_count(b as usize), 1);
        assert_eq!(gc::root_count(a as usize), 0);
        let u = crate::nova_rt_alloc(16);
        nova_rt_ui_state_store(u);
        assert_eq!(nova_rt_ui_state_load(), u);
        assert_eq!(nova_rt_router_state_load(), b);
    }
}
