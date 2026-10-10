//! The fixes the checker attaches to the errors it raises (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §4; plan decision 3).

use nova_diagnostics::{Edit, Fix, Span};

use super::Checker;

/// How a local the user wrote was bound (spec §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Binding {
    /// By a `let`.
    Let,
    /// As a function's, a method's or a closure's parameter, `self`
    /// included.
    Param,
    /// By a pattern or a `for` loop, where `mut` does not parse.
    Other,
}

impl<'a> Checker<'a> {
    /// "make `x` mutable" (spec §4.1): `mut ` before the name of a local
    /// bound by a `let` or as a parameter. `decl` is that name's span.
    /// Never for `self`, since changing a receiver changes every caller,
    /// nor in a module this project does not own (plan decision 19).
    pub(super) fn mutable_fix(&self, name: &str, decl: Span) -> Option<Fix> {
        if name == "self" || !self.defs.owned(self.cur_module) {
            return None;
        }
        match self.bindings.get(&decl) {
            Some(Binding::Let | Binding::Param) => Some(Fix::new(
                format!("make `{name}` mutable"),
                vec![Edit::insert(decl.start, decl.file, "mut ")],
            )),
            _ => None,
        }
    }

    /// Add `fix`, if there is one, to the diagnostic just pushed, unless
    /// this project does not own the module being checked (spec §3.1; plan
    /// decision 19).
    pub(super) fn push_fix(&mut self, fix: Option<Fix>) {
        if !self.defs.owned(self.cur_module) {
            return;
        }
        if let (Some(fix), Some(d)) = (fix, self.diagnostics.last_mut()) {
            d.fixes.push(fix);
        }
    }
}
